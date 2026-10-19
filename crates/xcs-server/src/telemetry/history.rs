//! Bounded, rebuildable monitoring cache; never part of storage publication.
use super::{Snapshot, statistics::Statistics};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, VecDeque},
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

pub(super) const RETENTION_SECONDS: i64 = 31 * 24 * 60 * 60;
const RECENT_SECONDS: i64 = 60 * 60;
const MAX_MINUTE_POINTS: usize = (RETENTION_SECONDS / 60) as usize + 2;
const MAX_RECENT_POINTS: usize = (RECENT_SECONDS / 5) as usize + 2;
const MAX_CACHE_FILE: u64 = 16 * 1024 * 1024;
const MAX_CACHE_LINE: usize = 64 * 1024;
const MAX_MEMORY_BYTES: usize = 128 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Sample {
    pub unix_seconds: i64,
    pub observations: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statistics: Option<Statistics>,
    pub status: Snapshot,
}

impl Sample {
    fn valid_at(&self, now: i64) -> bool {
        self.unix_seconds >= now - RETENTION_SECONDS
            && self.unix_seconds <= now
            && super::timestamp(self.unix_seconds).ok().as_deref()
                == Some(self.status.collected_at.as_str())
            && (1..=60).contains(&self.observations)
            && self
                .statistics
                .as_ref()
                .map_or(self.observations == 1, |statistics| {
                    statistics.valid_for(&self.status, self.observations)
                })
            && self.status.disks.len() <= 256
            && self
                .status
                .cpu
                .usage_percent
                .is_none_or(|value| value.is_finite() && (0.0..=100.0).contains(&value))
            && self
                .status
                .cpu
                .power_watts
                .is_none_or(|value| value.is_finite() && value >= 0.0)
            && self.status.disks.iter().all(|disk| {
                disk.power_watts
                    .is_none_or(|value| value.is_finite() && value >= 0.0)
            })
    }
}

pub(super) struct HistoryStore {
    directory: Option<PathBuf>,
    recent: VecDeque<Sample>,
    minutes: VecDeque<Sample>,
    pending_minute: Vec<Sample>,
    last_maintenance_minute: i64,
}

impl HistoryStore {
    pub(super) fn memory(now: i64) -> Self {
        Self {
            directory: None,
            recent: VecDeque::new(),
            minutes: VecDeque::new(),
            pending_minute: Vec::new(),
            last_maintenance_minute: now / 60,
        }
    }

    #[cfg(test)]
    pub(super) fn is_persistent(&self) -> bool {
        self.directory.is_some()
    }

    pub(super) fn open(directory: &Path, now: i64) -> io::Result<Self> {
        ensure_private_directory(directory)?;
        let mut result = Self {
            directory: Some(directory.into()),
            recent: VecDeque::new(),
            minutes: VecDeque::new(),
            pending_minute: Vec::new(),
            last_maintenance_minute: -1,
        };
        result.maintenance(now)?;
        let mut files = fs::read_dir(directory)?
            .filter_map(Result::ok)
            .filter(|entry| {
                cache_name(&entry.file_name().to_string_lossy())
                    .is_some_and(|(recent, bucket)| file_in_window(recent, bucket, now))
            })
            .take(36)
            .collect::<Vec<_>>();
        files.sort_by_key(|entry| std::cmp::Reverse(entry.file_name()));
        let mut recovery_bytes = 0_usize;
        let mut recent = BTreeMap::new();
        let mut minutes = BTreeMap::new();
        // At most 32 daily files and 2 recent hourly files pass admission.
        for entry in files {
            let Some((recent_file, bucket)) = cache_name(&entry.file_name().to_string_lossy())
            else {
                continue;
            };
            if !file_in_window(recent_file, bucket, now) {
                continue;
            }
            let metadata = entry.path().symlink_metadata()?;
            if !metadata.is_file()
                || metadata.file_type().is_symlink()
                || metadata.len() > MAX_CACHE_FILE
            {
                continue;
            }
            let Ok(bytes) = read_bounded_file(&entry.path()) else {
                continue;
            };
            for line in bytes.split(|byte| *byte == b'\n') {
                if line.is_empty()
                    || line.len() > MAX_CACHE_LINE
                    || recovery_bytes.saturating_add(line.len()) > MAX_MEMORY_BYTES
                {
                    continue;
                }
                let Ok(sample) = serde_json::from_slice::<Sample>(line) else {
                    continue;
                };
                if !sample.valid_at(now) {
                    continue;
                }
                recovery_bytes += line.len();
                if recent_file
                    && sample.unix_seconds >= now - RECENT_SECONDS
                    && sample.unix_seconds / 3600 == bucket
                {
                    recent.insert(sample.unix_seconds, sample);
                } else if !recent_file && sample.unix_seconds / 86400 == bucket {
                    minutes.insert(sample.unix_seconds / 60, sample);
                }
            }
        }
        result.recent = recent.into_values().collect();
        result.minutes = minutes.into_values().collect();
        // A restart can interrupt the last minute. Recover completed minute
        // aggregates from real recent samples without creating any missing data.
        let mut recovered = BTreeMap::<i64, Vec<Sample>>::new();
        for sample in &result.recent {
            recovered
                .entry(sample.unix_seconds / 60)
                .or_default()
                .push(sample.clone());
        }
        for (minute, samples) in recovered {
            if minute == now / 60 {
                result.pending_minute = samples;
            } else if !result
                .minutes
                .iter()
                .any(|sample| sample.unix_seconds / 60 == minute)
            {
                let sample = average(&samples);
                if let Err(error) = result.append_file(false, &sample) {
                    tracing::warn!(%error, "monitoring cache recovery is read-only; using bounded memory history");
                    result.directory = None;
                }
                result.minutes.push_back(sample);
            }
        }
        result
            .minutes
            .make_contiguous()
            .sort_by_key(|sample| sample.unix_seconds);
        result.last_maintenance_minute = -1;
        result.prune(now);
        result.last_maintenance_minute = now / 60;
        Ok(result)
    }

    pub(super) fn record(&mut self, sample: Sample) -> io::Result<()> {
        let now = sample.unix_seconds;
        if !sample.valid_at(now) {
            return Err(io::Error::other("invalid telemetry sample"));
        }
        // Repeated observations in the same second never multiply history points.
        if self
            .recent
            .back()
            .is_some_and(|previous| previous.unix_seconds >= now)
        {
            return Ok(());
        }
        let mut error = None;
        if self
            .pending_minute
            .first()
            .is_some_and(|pending| pending.unix_seconds / 60 != now / 60)
        {
            let minute = average(&self.pending_minute);
            if let Err(failure) = self.append_file(false, &minute) {
                error = Some(failure);
            }
            self.minutes.push_back(minute);
            self.pending_minute.clear();
        }
        self.pending_minute.push(sample.clone());
        if let Err(failure) = self.append_file(true, &sample) {
            error = Some(failure);
        }
        self.recent.push_back(sample);
        self.prune(now);
        if error.is_some() {
            self.directory = None;
        }
        if self.last_maintenance_minute != now / 60 {
            if let Err(failure) = self.maintenance(now) {
                error = Some(failure);
            }
        }
        match error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    pub(super) fn samples(&self, from: i64, to: i64, interval: i64) -> Vec<Snapshot> {
        let mut source = BTreeMap::<i64, Sample>::new();
        // Recent full samples replace their minute aggregate; they are never
        // averaged twice merely because two cache resolutions cover one period.
        let recent_minutes = self
            .recent
            .iter()
            .filter(|sample| sample.unix_seconds >= from && sample.unix_seconds <= to)
            .map(|sample| sample.unix_seconds / 60)
            .collect::<std::collections::BTreeSet<_>>();
        for sample in &self.minutes {
            if sample.unix_seconds >= from
                && sample.unix_seconds <= to
                && !recent_minutes.contains(&(sample.unix_seconds / 60))
            {
                source.insert(sample.unix_seconds, sample.clone());
            }
        }
        for sample in &self.recent {
            if sample.unix_seconds >= from && sample.unix_seconds <= to {
                source.insert(sample.unix_seconds, sample.clone());
            }
        }
        let mut buckets = BTreeMap::<i64, Vec<Sample>>::new();
        for sample in source.into_values() {
            buckets
                .entry((sample.unix_seconds - from) / interval)
                .or_default()
                .push(sample);
        }
        buckets
            .into_values()
            .map(|samples| average(&samples).status)
            .collect()
    }

    fn prune(&mut self, now: i64) {
        while self
            .recent
            .front()
            .is_some_and(|sample| sample.unix_seconds < now - RECENT_SECONDS)
            || self.recent.len() > MAX_RECENT_POINTS
        {
            self.recent.pop_front();
        }
        while self
            .minutes
            .front()
            .is_some_and(|sample| sample.unix_seconds < now - RETENTION_SECONDS)
            || self.minutes.len() > MAX_MINUTE_POINTS
        {
            self.minutes.pop_front();
        }
        let approximate = |points: &VecDeque<Sample>| {
            points
                .iter()
                .map(|sample| {
                    serde_json::to_vec(sample).map_or(MAX_CACHE_LINE, |bytes| bytes.len())
                })
                .sum::<usize>()
        };
        // Check memory admission only once per minute; the point-count bound is
        // enforced on every observation. 3-drive installations retain 31 days.
        if self.last_maintenance_minute != now / 60 {
            let mut bytes = approximate(&self.minutes) + approximate(&self.recent);
            while bytes > MAX_MEMORY_BYTES && !self.minutes.is_empty() {
                if let Some(sample) = self.minutes.pop_front() {
                    bytes = bytes.saturating_sub(
                        serde_json::to_vec(&sample).map_or(MAX_CACHE_LINE, |value| value.len()),
                    );
                }
            }
            while bytes > MAX_MEMORY_BYTES && self.recent.len() > 1 {
                if let Some(sample) = self.recent.pop_front() {
                    bytes = bytes.saturating_sub(
                        serde_json::to_vec(&sample).map_or(MAX_CACHE_LINE, |value| value.len()),
                    );
                }
            }
        }
    }

    fn append_file(&self, recent: bool, sample: &Sample) -> io::Result<()> {
        let Some(directory) = &self.directory else {
            return Ok(());
        };
        let bytes = serde_json::to_vec(sample)?;
        if bytes.len() > MAX_CACHE_LINE {
            return Err(io::Error::other("telemetry sample exceeds cache admission"));
        }
        let bucket = sample.unix_seconds / if recent { 3600 } else { 86400 };
        let name = format!(
            "{}-{bucket}.jsonl",
            if recent { "recent" } else { "minute" }
        );
        let path = directory.join(name);
        let mut options = OpenOptions::new();
        options.append(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
            #[cfg(target_os = "linux")]
            options.custom_flags(0x20000 | 0x800); // O_NOFOLLOW | O_NONBLOCK
        }
        let mut file = options.open(&path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.len() + bytes.len() as u64 + 1 > MAX_CACHE_FILE {
            return Err(io::Error::other("telemetry cache file exceeds admission"));
        }
        file.write_all(&bytes)?;
        file.write_all(b"\n")?;
        Ok(())
    }

    fn maintenance(&mut self, now: i64) -> io::Result<()> {
        let Some(directory) = &self.directory else {
            self.last_maintenance_minute = now / 60;
            return Ok(());
        };
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let Some((recent, bucket)) = cache_name(&entry.file_name().to_string_lossy()) else {
                continue;
            };
            let metadata = entry.path().symlink_metadata()?;
            if metadata.is_file()
                && !metadata.file_type().is_symlink()
                && !file_in_window(recent, bucket, now)
            {
                fs::remove_file(entry.path())?;
            }
        }
        self.last_maintenance_minute = now / 60;
        Ok(())
    }
}

fn cache_name(name: &str) -> Option<(bool, i64)> {
    let (kind, bucket) = name.strip_suffix(".jsonl")?.split_once('-')?;
    let recent = match kind {
        "recent" => true,
        "minute" => false,
        _ => return None,
    };
    if bucket.is_empty() || !bucket.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let parsed = bucket.parse::<i64>().ok()?;
    if parsed.to_string() != bucket {
        return None;
    }
    Some((recent, parsed))
}

fn file_in_window(recent: bool, bucket: i64, now: i64) -> bool {
    let seconds = if recent { 3600 } else { 86400 };
    let retention = if recent {
        RECENT_SECONDS
    } else {
        RETENTION_SECONDS
    };
    ((now - retention) / seconds..=now / seconds).contains(&bucket)
}

fn ensure_private_directory(directory: &Path) -> io::Result<()> {
    match directory.symlink_metadata() {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "telemetry cache must not be a symbolic link",
            ));
        }
        Ok(metadata) if !metadata.is_dir() => {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "telemetry cache directory is occupied",
            ));
        }
        Ok(_) => (),
        Err(error) if error.kind() == io::ErrorKind::NotFound => fs::create_dir(directory)?,
        Err(error) => return Err(error),
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn read_bounded_file(path: &Path) -> io::Result<Vec<u8>> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(0x20000 | 0x800);
    }
    let file = options.open(path)?;
    if !file.metadata()?.is_file() {
        return Ok(Vec::new());
    }
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(
        &mut std::io::Read::take(file, MAX_CACHE_FILE + 1),
        &mut bytes,
    )?;
    if bytes.len() as u64 > MAX_CACHE_FILE {
        return Ok(Vec::new());
    }
    Ok(bytes)
}

pub(super) fn average(samples: &[Sample]) -> Sample {
    let mut result = samples
        .last()
        .expect("nonempty telemetry aggregate")
        .clone();
    result.observations = samples.iter().map(|sample| sample.observations).sum();
    let mut statistics = Statistics::default();
    let mut disks = BTreeMap::new();
    for sample in samples {
        statistics.merge(
            &sample
                .statistics
                .clone()
                .unwrap_or_else(|| Statistics::observation(&sample.status)),
        );
        for disk in &sample.status.disks {
            disks.insert(disk.id.clone(), disk.clone());
        }
    }
    result.status.disks = disks.into_values().collect();
    statistics.apply(&mut result.status);
    result.statistics = Some(statistics);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::{CpuStatus, MemoryStatus};

    fn sample(at: i64, value: Option<f64>) -> Sample {
        Sample {
            unix_seconds: at,
            observations: 1,
            statistics: None,
            status: Snapshot {
                collected_at: super::super::timestamp(at).unwrap(),
                cpu: CpuStatus {
                    usage_percent: value.map(|value| value.min(100.0) as f32),
                    logical_count: 4,
                    power_watts: value,
                    power_unavailable_reason: value.is_none().then(|| "sensor_unavailable".into()),
                },
                memory: MemoryStatus {
                    total_bytes: Some("18446744073709551615".into()),
                    used_bytes: Some("10".into()),
                    available_bytes: Some("18446744073709551605".into()),
                },
                disks: vec![],
            },
        }
    }

    #[test]
    fn restart_recovers_real_samples_and_keeps_missing_values_absent() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("cache");
        let now = 1_800_000_120;
        let mut cache = HistoryStore::open(&path, now).unwrap();
        for offset in (0..80).step_by(5) {
            cache.record(sample(now + offset, None)).unwrap();
        }
        drop(cache);
        let cache = HistoryStore::open(&path, now + 80).unwrap();
        let result = cache.samples(now, now + 80, 5);
        assert_eq!(result.len(), 16);
        assert!(
            result.iter().all(
                |sample| sample.cpu.power_watts.is_none() && sample.cpu.usage_percent.is_none()
            )
        );
        assert_eq!(
            result[0].memory.total_bytes.as_deref(),
            Some("18446744073709551615")
        );
        assert_eq!(cache.minutes.len(), 1);
    }

    #[test]
    fn range_averages_real_observations_without_duplicate_resolution_or_fake_gaps() {
        let root = tempfile::tempdir().unwrap();
        let now = 1_800_000_120;
        let mut cache = HistoryStore::open(&root.path().join("cache"), now).unwrap();
        for offset in (0..180).step_by(5) {
            cache
                .record(sample(now + offset, Some(offset as f64)))
                .unwrap();
        }
        let result = cache.samples(now, now + 180, 60);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].cpu.power_watts, Some(27.5));
        assert_eq!(result[1].cpu.power_watts, Some(87.5));
        assert_eq!(result[2].cpu.power_watts, Some(147.5));
        assert!(cache.samples(now + 300, now + 600, 5).is_empty());
    }

    #[test]
    fn cleanup_only_removes_recognized_expired_regular_cache_files() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("cache");
        fs::create_dir(&path).unwrap();
        fs::write(path.join("minute-1.jsonl"), b"expired").unwrap();
        fs::write(path.join("keep.txt"), b"foreign").unwrap();
        fs::create_dir(path.join("recent-1.jsonl")).unwrap();
        let now = 1_800_000_120;
        let cache = HistoryStore::open(&path, now).unwrap();
        assert!(cache.recent.is_empty());
        assert!(!path.join("minute-1.jsonl").exists());
        assert!(path.join("keep.txt").exists());
        assert!(path.join("recent-1.jsonl").is_dir());
    }

    #[test]
    fn month_range_stays_bounded_and_weights_multiple_resolutions_by_real_observations() {
        let root = tempfile::tempdir().unwrap();
        let now = 1_800_000_120;
        let mut cache = HistoryStore::open(&root.path().join("cache"), now).unwrap();
        for at in ((now - RETENTION_SECONDS - 120)..=now).step_by(60) {
            let mut point = sample(at, Some(20.0));
            point = average(&vec![point; 12]);
            cache.minutes.push_back(point);
        }
        cache.prune(now);
        assert!(cache.minutes.len() <= MAX_MINUTE_POINTS);
        assert!(cache.minutes.front().unwrap().unix_seconds >= now - RETENTION_SECONDS);
        let result = cache.samples(now - 30 * 86400, now, 7200);
        assert!(result.len() <= 361);
        assert!(
            result
                .iter()
                .all(|point| point.cpu.power_watts == Some(20.0))
        );
        let mut minute = sample(now - 60, Some(20.0));
        minute = average(&vec![minute; 12]);
        let actual = sample(now, Some(0.0));
        assert_eq!(
            average(&[minute, actual]).status.cpu.power_watts,
            Some(240.0 / 13.0)
        );
    }

    #[test]
    fn mixed_null_metrics_remain_exact_after_two_aggregations_and_persistence() {
        let root = tempfile::tempdir().unwrap();
        let now = 1_800_000_120;
        let metric = |at, value: Option<f64>| {
            let mut sample = sample(at, value);
            let bytes = value.map(|value| (value as u64).to_string());
            sample.status.memory.total_bytes = bytes.clone();
            sample.status.memory.used_bytes = bytes.clone();
            sample.status.memory.available_bytes = bytes.clone();
            sample.status.disks = vec![super::super::DiskStatus {
                id: "P01".into(),
                name: "P01".into(),
                online: true,
                read_only: false,
                simulation: false,
                total_bytes: bytes.clone(),
                used_bytes: bytes.clone(),
                available_bytes: bytes,
                usage_percent: value,
                power_watts: value,
                power_unavailable_reason: value.is_none().then(|| "sensor_unavailable".into()),
            }];
            sample
        };
        let a = (0..12)
            .map(|i| metric(now + i * 5, (i == 0).then_some(100.0)))
            .collect::<Vec<_>>();
        let b = (0..12)
            .map(|i| metric(now + 60 + i * 5, Some(0.0)))
            .collect::<Vec<_>>();
        let minute_a = average(&a);
        let minute_b = average(&b);
        assert_eq!(minute_a.observations, 12);
        assert!(minute_a.valid_at(now + 120));
        assert!(minute_b.valid_at(now + 120));
        // Round trip through the actual cache serializer before recombining.
        let cache = HistoryStore::open(&root.path().join("cache"), now + 120).unwrap();
        cache.append_file(false, &minute_a).unwrap();
        cache.append_file(false, &minute_b).unwrap();
        drop(cache);
        let mut cache = HistoryStore::open(&root.path().join("cache"), now + 120).unwrap();
        assert_eq!(cache.minutes.len(), 2);
        let twice = average(cache.minutes.make_contiguous());
        let direct = average(&a.into_iter().chain(b).collect::<Vec<_>>());
        let expected = 100.0 / 13.0;
        assert!((twice.status.cpu.usage_percent.unwrap() - expected as f32).abs() < 0.00001);
        assert_eq!(twice.status.cpu.power_watts, Some(expected));
        assert_eq!(twice.status.memory.used_bytes.as_deref(), Some("7"));
        assert_eq!(twice.status.memory.total_bytes.as_deref(), Some("7"));
        assert_eq!(twice.status.memory.available_bytes.as_deref(), Some("7"));
        assert_eq!(twice.status.disks[0].usage_percent, Some(expected));
        assert_eq!(twice.status.disks[0].power_watts, Some(expected));
        assert_eq!(twice.status.disks[0].used_bytes.as_deref(), Some("7"));
        assert_eq!(
            serde_json::to_value(twice.status).unwrap(),
            serde_json::to_value(direct.status).unwrap()
        );
    }

    #[test]
    fn byte_statistics_round_trip_u128_sums_and_avoid_repeated_integer_rounding() {
        let now = 1_800_000_120;
        let mut a = sample(now, Some(1.0));
        a.status.memory.used_bytes = Some("0".into());
        let mut b = sample(now + 5, Some(1.0));
        b.status.memory.used_bytes = Some("1".into());
        let minute = average(&[a, b]);
        assert_eq!(minute.status.memory.used_bytes.as_deref(), Some("0"));
        // total_bytes is u64::MAX per observation, so its serialized sum is
        // larger than u64. The cache must preserve that u128 integer exactly.
        let encoded = serde_json::to_vec(&minute).unwrap();
        assert!(String::from_utf8_lossy(&encoded).contains("36893488147419103230"));
        let restored: Sample = serde_json::from_slice(&encoded).unwrap();
        assert!(restored.valid_at(now + 60));
        let mut c = sample(now + 60, Some(1.0));
        c.status.memory.used_bytes = Some("2".into());
        let combined = average(&[restored, c]);
        assert_eq!(combined.status.memory.used_bytes.as_deref(), Some("1"));
        assert_eq!(
            combined.status.memory.total_bytes.as_deref(),
            Some("18446744073709551615")
        );
    }

    #[test]
    fn old_aggregates_without_metric_statistics_are_ignored_but_raw_points_recover() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("cache");
        let now = 1_800_000_120;
        let cache = HistoryStore::open(&path, now).unwrap();
        let mut old = sample(now - 60, Some(100.0));
        old.observations = 12;
        assert!(!old.valid_at(now));
        cache.append_file(false, &old).unwrap();
        cache
            .append_file(true, &sample(now - 55, Some(20.0)))
            .unwrap();
        drop(cache);
        let cache = HistoryStore::open(&path, now).unwrap();
        assert_eq!(cache.minutes.len(), 1);
        assert_eq!(cache.minutes[0].status.cpu.power_watts, Some(20.0));
        assert_eq!(cache.minutes[0].observations, 1);
    }

    #[test]
    fn recovery_ignores_partial_invalid_and_future_samples() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("cache");
        let now = 1_800_000_120;
        let mut cache = HistoryStore::open(&path, now).unwrap();
        cache.record(sample(now, Some(1.0))).unwrap();
        drop(cache);
        let path_file = path.join(format!("recent-{}.jsonl", now / 3600));
        let mut file = OpenOptions::new().append(true).open(path_file).unwrap();
        writeln!(file, "{{not-json}}").unwrap();
        writeln!(
            file,
            "{}",
            serde_json::to_string(&sample(now + 100, Some(2.0))).unwrap()
        )
        .unwrap();
        write!(file, "{{\"unix_seconds\":").unwrap();
        drop(file);
        let cache = HistoryStore::open(&path, now + 1).unwrap();
        assert_eq!(cache.samples(now, now + 1, 5).len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn symlink_cache_directory_and_files_are_not_followed() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("target");
        fs::create_dir(&target).unwrap();
        symlink(&target, root.path().join("linked")).unwrap();
        assert!(HistoryStore::open(&root.path().join("linked"), 1_800_000_120).is_err());
        let path = root.path().join("cache");
        let mut cache = HistoryStore::open(&path, 1_800_000_120).unwrap();
        let external = root.path().join("external");
        fs::write(&external, b"untouched").unwrap();
        symlink(
            &external,
            path.join(format!("recent-{}.jsonl", 1_800_000_120 / 3600)),
        )
        .unwrap();
        assert!(cache.record(sample(1_800_000_120, Some(1.0))).is_err());
        assert_eq!(fs::read(&external).unwrap(), b"untouched");
        assert_eq!(cache.samples(1_800_000_120, 1_800_000_120, 5).len(), 1);
    }
}
