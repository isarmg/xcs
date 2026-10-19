//! Read-only Linux sensor support. Never infer power from utilization or TDP.
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

#[derive(Clone, Debug, Default)]
pub(super) struct PowerReading {
    pub watts: Option<f64>,
    pub reason: Option<String>,
}
impl PowerReading {
    pub(super) fn unavailable(reason: &str) -> Self {
        Self {
            watts: None,
            reason: Some(reason.into()),
        }
    }
    fn measured(watts: f64) -> Self {
        Self {
            watts: Some(watts),
            reason: None,
        }
    }
}

#[derive(Clone)]
struct Energy {
    microjoules: u64,
    range: Option<u64>,
}

pub(super) struct CpuPowerSampler {
    root: PathBuf,
    previous: Option<(Instant, BTreeMap<PathBuf, Energy>)>,
}
impl CpuPowerSampler {
    pub(super) fn new() -> Self {
        Self {
            root: PathBuf::from("/sys/class/powercap"),
            previous: None,
        }
    }

    pub(super) fn sample(&mut self) -> PowerReading {
        self.sample_at(Instant::now())
    }

    fn sample_at(&mut self, now: Instant) -> PowerReading {
        let mut zones = BTreeSet::new();
        discover_packages(&self.root, 0, &mut zones, &mut BTreeSet::new());
        if zones.is_empty() {
            self.previous = None;
            return PowerReading::unavailable("sensor_unavailable");
        }
        let mut current = BTreeMap::new();
        for zone in zones {
            let Some(microjoules) = read_u64(&zone.join("energy_uj")) else {
                self.previous = None;
                return PowerReading::unavailable("sensor_unreadable");
            };
            current.insert(
                zone.clone(),
                Energy {
                    microjoules,
                    range: read_u64(&zone.join("max_energy_range_uj")).filter(|v| *v > 0),
                },
            );
        }
        let previous = self.previous.replace((now, current.clone()));
        let Some((then, previous)) = previous else {
            return PowerReading::unavailable("sampling");
        };
        let seconds = now.saturating_duration_since(then).as_secs_f64();
        if seconds <= 0.0 || previous.keys().ne(current.keys()) {
            return PowerReading::unavailable("sampling");
        }
        let mut energy = 0_u128;
        for (path, next) in &current {
            let old = &previous[path];
            let Some(delta) = energy_delta(old, next) else {
                return PowerReading::unavailable("counter_reset");
            };
            energy += u128::from(delta);
        }
        PowerReading::measured(energy as f64 / 1_000_000.0 / seconds)
    }
}

fn energy_delta(previous: &Energy, current: &Energy) -> Option<u64> {
    if current.range != previous.range {
        return None;
    }
    if current.microjoules >= previous.microjoules {
        return current.microjoules.checked_sub(previous.microjoules);
    }
    let range = current.range?;
    if previous.microjoules > range || current.microjoules > range {
        return None;
    }
    range
        .checked_sub(previous.microjoules)?
        .checked_add(current.microjoules)
}

fn discover_packages(
    root: &Path,
    depth: usize,
    result: &mut BTreeSet<PathBuf>,
    seen: &mut BTreeSet<PathBuf>,
) {
    if depth > 4 || seen.len() >= 256 {
        return;
    }
    let Ok(canonical) = root.canonicalize() else {
        return;
    };
    if !seen.insert(canonical.clone()) {
        return;
    }
    if fs::read_to_string(canonical.join("name"))
        .ok()
        .is_some_and(|name| {
            name.trim().strip_prefix("package-").is_some_and(|suffix| {
                !suffix.is_empty() && suffix.bytes().all(|c| c.is_ascii_digit())
            })
        })
    {
        result.insert(canonical);
        // A package already contains its core/uncore domains: do not add them twice.
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.take(256).flatten() {
        if entry.path().is_dir() {
            discover_packages(&entry.path(), depth + 1, result, seen);
        }
    }
}

fn read_u64(path: &Path) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

/// The storage engine provides a previously verified physical sysfs identity.
/// A sensor must belong to that drive (or its own SCSI device), not merely to
/// the same PCI controller or motherboard. Several power rails are ambiguous;
/// without a documented total channel we deliberately report no measurement.
#[cfg(any(feature = "core", test))]
pub(crate) fn disk_power(
    physical_device: &str,
    simulation: bool,
    online: bool,
) -> (Option<f64>, Option<String>) {
    let reading = if !online {
        PowerReading::unavailable("disk_offline")
    } else if simulation {
        PowerReading::unavailable("simulated_disk")
    } else {
        read_disk_power(
            Path::new(physical_device),
            Path::new("/sys/class/hwmon"),
            Some(Path::new("/sys/devices")),
        )
    };
    (reading.watts, reading.reason)
}

#[cfg(any(feature = "core", test))]
fn read_disk_power(
    physical_device: &Path,
    hwmon_root: &Path,
    allowed_root: Option<&Path>,
) -> PowerReading {
    let Ok(physical) = physical_device.canonicalize() else {
        return PowerReading::unavailable("sensor_unavailable");
    };
    if allowed_root.is_some_and(|root| !physical.starts_with(root)) {
        return PowerReading::unavailable("sensor_unavailable");
    }
    let mut owners = vec![physical.clone()];
    if let Ok(device) = physical.join("device").canonicalize() {
        owners.push(device);
    }
    let Ok(entries) = fs::read_dir(hwmon_root) else {
        return PowerReading::unavailable("sensor_unavailable");
    };
    let mut channels = BTreeMap::<(PathBuf, String), PathBuf>::new();
    for entry in entries.take(256).flatten() {
        let path = entry.path();
        let owner = path.join("device").canonicalize().ok().or_else(|| {
            let canonical = path.canonicalize().ok()?;
            canonical.parent()?.parent().map(Path::to_path_buf)
        });
        let Some(owner) = owner else {
            continue;
        };
        if !owners
            .iter()
            .any(|drive| owner == *drive || owner.starts_with(drive))
        {
            continue;
        }
        let Ok(files) = fs::read_dir(&path) else {
            continue;
        };
        for file in files.take(512).flatten() {
            let name = file.file_name().to_string_lossy().into_owned();
            let Some((stem, kind)) = name.rsplit_once('_') else {
                continue;
            };
            if !matches!(kind, "input" | "average")
                || !stem
                    .strip_prefix("power")
                    .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
            {
                continue;
            }
            let Ok(sensor) = path.canonicalize() else {
                continue;
            };
            let key = (sensor, stem.to_string());
            let candidate = file.path();
            if kind == "input" || !channels.contains_key(&key) {
                channels.insert(key, candidate);
            }
        }
    }
    if channels.len() > 1 {
        return PowerReading::unavailable("sensor_ambiguous");
    }
    let Some(path) = channels.into_values().next() else {
        return PowerReading::unavailable("sensor_unavailable");
    };
    match read_u64(&path) {
        Some(microwatts) => PowerReading::measured(microwatts as f64 / 1_000_000.0),
        None => PowerReading::unavailable("sensor_unreadable"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn write(path: &Path, value: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, value).unwrap();
    }

    #[test]
    fn cpu_uses_energy_differences_and_avoids_counting_subzones_twice() {
        let root = tempfile::tempdir().unwrap();
        let a = root.path().join("intel-rapl:0");
        let b = root.path().join("intel-rapl:1");
        for (p, name) in [(&a, "package-0"), (&b, "package-1")] {
            write(&p.join("name"), name);
            write(&p.join("energy_uj"), "1000000");
            write(&p.join("max_energy_range_uj"), "1000000000");
        }
        write(&a.join("core/name"), "core");
        write(&a.join("core/energy_uj"), "999999999");
        let mut sampler = CpuPowerSampler {
            root: root.path().into(),
            previous: None,
        };
        let start = Instant::now();
        assert_eq!(sampler.sample_at(start).reason.as_deref(), Some("sampling"));
        write(&a.join("energy_uj"), "11000000");
        write(&b.join("energy_uj"), "21000000");
        assert_eq!(
            sampler.sample_at(start + Duration::from_secs(5)).watts,
            Some(6.0)
        );
        fs::remove_file(b.join("energy_uj")).unwrap();
        assert_eq!(
            sampler
                .sample_at(start + Duration::from_secs(10))
                .reason
                .as_deref(),
            Some("sensor_unreadable")
        );
    }

    #[test]
    fn wrap_and_missing_counter_range_are_distinct() {
        let e = |microjoules, range| Energy { microjoules, range };
        assert_eq!(
            energy_delta(&e(900, Some(1000)), &e(100, Some(1000))),
            Some(200)
        );
        assert_eq!(energy_delta(&e(900, None), &e(100, None)), None);
        assert_eq!(energy_delta(&e(900, Some(1000)), &e(100, Some(2000))), None);
    }

    #[cfg(unix)]
    #[test]
    fn disk_sensor_requires_drive_ownership_and_does_not_sum_unknown_rails() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let drive = root.path().join("devices/nvme0");
        fs::create_dir_all(&drive).unwrap();
        let unrelated = root.path().join("devices/mainboard");
        fs::create_dir_all(&unrelated).unwrap();
        let hwmon = root.path().join("hwmon");
        write(&hwmon.join("hwmon0/power1_input"), "999000000");
        symlink(&unrelated, hwmon.join("hwmon0/device")).unwrap();
        assert_eq!(
            read_disk_power(&drive, &hwmon, None).reason.as_deref(),
            Some("sensor_unavailable")
        );
        write(&hwmon.join("hwmon1/power1_input"), "4200000");
        write(&hwmon.join("hwmon1/power1_average"), "4100000");
        symlink(&drive, hwmon.join("hwmon1/device")).unwrap();
        assert_eq!(read_disk_power(&drive, &hwmon, None).watts, Some(4.2));
        write(&hwmon.join("hwmon1/power2_input"), "1500000");
        assert_eq!(
            read_disk_power(&drive, &hwmon, None).reason.as_deref(),
            Some("sensor_ambiguous")
        );
        assert_eq!(
            disk_power("ignored", true, true).1.as_deref(),
            Some("simulated_disk")
        );
    }
}
