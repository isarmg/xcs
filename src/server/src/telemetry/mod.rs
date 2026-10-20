//! Host monitoring with a private periodic sampler and a rebuildable cache.
mod history;
mod power;
mod statistics;

use crate::{ApiError, AppState, Backend, auth::Identity};
use axum::{
    Extension, Json,
    extract::{Query, State},
};
use history::{HistoryStore, Sample};
use power::CpuPowerSampler;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

#[cfg(feature = "core")]
pub(crate) use power::disk_power;
const SAMPLE_INTERVAL: Duration = Duration::from_secs(5);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct CpuStatus {
    usage_percent: Option<f32>,
    logical_count: usize,
    power_watts: Option<f64>,
    power_unavailable_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct MemoryStatus {
    total_bytes: Option<String>,
    used_bytes: Option<String>,
    available_bytes: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct DiskStatus {
    id: String,
    name: String,
    online: bool,
    read_only: bool,
    simulation: bool,
    total_bytes: Option<String>,
    available_bytes: Option<String>,
    used_bytes: Option<String>,
    #[serde(default)]
    usage_percent: Option<f64>,
    #[serde(default)]
    power_watts: Option<f64>,
    #[serde(default)]
    power_unavailable_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Snapshot {
    collected_at: String,
    cpu: CpuStatus,
    memory: MemoryStatus,
    disks: Vec<DiskStatus>,
}

/// One object per service retains true interval baselines. First CPU usage and
/// RAPL power observations are unavailable, rather than fabricated zeroes.
struct SystemSampler {
    system: System,
    last_refresh: Instant,
    cpu_usage: Option<f32>,
    power: CpuPowerSampler,
    cpu_power: power::PowerReading,
}

impl SystemSampler {
    fn new() -> Self {
        let mut power = CpuPowerSampler::new();
        let cpu_power = power.sample();
        Self {
            system: System::new_with_specifics(
                RefreshKind::nothing()
                    .with_cpu(CpuRefreshKind::nothing().with_cpu_usage())
                    .with_memory(MemoryRefreshKind::everything()),
            ),
            last_refresh: Instant::now(),
            cpu_usage: None,
            power,
            cpu_power,
        }
    }

    fn sample(&mut self) -> (CpuStatus, MemoryStatus) {
        if self.last_refresh.elapsed() >= sysinfo::MINIMUM_CPU_UPDATE_INTERVAL {
            self.system.refresh_cpu_usage();
            self.system.refresh_memory();
            self.cpu_usage = normalized_cpu_usage(self.system.global_cpu_usage());
            self.cpu_power = self.power.sample();
            self.last_refresh = Instant::now();
        }
        let total = self.system.total_memory();
        let available = self.system.available_memory().min(total);
        (
            CpuStatus {
                usage_percent: self.cpu_usage,
                logical_count: self.system.cpus().len(),
                power_watts: self.cpu_power.watts,
                power_unavailable_reason: self.cpu_power.reason.clone(),
            },
            memory_status(total, available),
        )
    }
}

fn normalized_cpu_usage(value: f32) -> Option<f32> {
    value
        .is_finite()
        .then(|| (value.clamp(0.0, 100.0) * 10.0).round() / 10.0)
}

fn memory_status(total: u64, available: u64) -> MemoryStatus {
    let available = available.min(total);
    MemoryStatus {
        total_bytes: (total > 0).then(|| total.to_string()),
        used_bytes: (total > 0).then(|| total.saturating_sub(available).to_string()),
        available_bytes: (total > 0).then(|| available.to_string()),
    }
}

pub(crate) struct Telemetry {
    // This private handle only executes the fixed, read-only monitor_disks
    // action; HTTP and protocol adapters retain AuthorizedBackend normally.
    backend: Arc<dyn Backend>,
    sampler: Mutex<SystemSampler>,
    history: Mutex<HistoryStore>,
}

impl Telemetry {
    pub(crate) fn start(
        backend: Arc<dyn Backend>,
        directory: &Path,
    ) -> Result<Arc<Self>, ApiError> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        let service = Arc::new(Self {
            backend,
            sampler: Mutex::new(SystemSampler::new()),
            history: Mutex::new(match HistoryStore::open(directory, now) {
                Ok(history) => history,
                Err(error) if error.kind() == std::io::ErrorKind::InvalidInput => {
                    return Err(ApiError::io(error));
                }
                Err(error) => {
                    tracing::warn!(%error, "monitoring cache unavailable; using bounded memory history");
                    HistoryStore::memory(now)
                }
            }),
        });
        // Synchronous initialization guarantees a usable first snapshot. A
        // backend without monitor support can still serve its other routes.
        if let Err(error) = service.refresh() {
            tracing::warn!(code = %error.code, "initial host sample is unavailable");
        }
        let weak = Arc::downgrade(&service);
        std::thread::Builder::new()
            .name("zxs-telemetry".into())
            .spawn(move || {
                loop {
                    std::thread::sleep(SAMPLE_INTERVAL);
                    let Some(service) = weak.upgrade() else {
                        break;
                    };
                    if let Err(error) = service.refresh() {
                        tracing::warn!(code = %error.code, "periodic host sample is unavailable");
                    }
                    // Release the service while sleeping. Dropping its router
                    // stops the sampler without retaining storage or auth state.
                }
            })
            .map_err(ApiError::io)?;
        Ok(service)
    }

    fn collect(&self) -> Result<Sample, ApiError> {
        let identity = Identity {
            principal: "zxs-system-telemetry".into(),
            admin: true,
            repositories: Default::default(),
        };
        let disks = self
            .backend
            .call(crate::BackendAction::MonitorDisks, &identity, json!({}))?;
        let disks = serde_json::from_value::<Vec<DiskStatus>>(disks["disks"].clone())
            .map_err(|_| unavailable("disk sampling is unavailable"))?;
        let (cpu, memory) = self
            .sampler
            .lock()
            .map_err(|_| unavailable("host sampling is unavailable"))?
            .sample();
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        Ok(Sample {
            unix_seconds: now,
            observations: 1,
            statistics: None,
            status: Snapshot {
                collected_at: timestamp(now)?,
                cpu,
                memory,
                disks,
            },
        })
    }

    fn refresh(&self) -> Result<(), ApiError> {
        let sample = self.collect()?;
        if let Err(error) = self
            .history
            .lock()
            .map_err(|_| unavailable("history sampling is unavailable"))?
            .record(sample)
        {
            // Cache failure cannot stop live monitoring or business storage.
            tracing::warn!(%error, "monitoring cache persistence failed; current samples remain in memory");
        }
        Ok(())
    }

    fn current(&self) -> Result<Snapshot, ApiError> {
        // Explicit refresh observes current host counters, while history stays
        // independently paced by the background sampler.
        self.collect().map(|sample| sample.status)
    }
}

fn unavailable(message: &str) -> ApiError {
    ApiError::new(503, "MONITOR_UNAVAILABLE", message)
}

fn timestamp(unix_seconds: i64) -> Result<String, ApiError> {
    time::OffsetDateTime::from_unix_timestamp(unix_seconds)
        .map_err(|_| unavailable("sample time is unavailable"))?
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|_| unavailable("sample time is unavailable"))
}

pub(crate) async fn status(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
) -> Result<Json<Value>, ApiError> {
    identity.require_admin()?;
    tokio::task::spawn_blocking(move || {
        state.telemetry.current().and_then(|status| {
            serde_json::to_value(status)
                .map(Json)
                .map_err(|_| unavailable("host sampling is unavailable"))
        })
    })
    .await
    .map_err(|_| unavailable("host sampling was interrupted"))?
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HistoryQuery {
    #[serde(default = "default_range")]
    range: String,
}
fn default_range() -> String {
    "1min".into()
}

fn range_parameters(range: &str) -> Result<(i64, i64), ApiError> {
    match range {
        "1min" => Ok((60, 5)),
        "30min" => Ok((30 * 60, 5)),
        "1h" => Ok((60 * 60, 10)),
        "24h" => Ok((24 * 60 * 60, 4 * 60)),
        "1month" => Ok((30 * 24 * 60 * 60, 2 * 60 * 60)),
        _ => Err(ApiError::invalid(
            "history range must be 1min, 30min, 1h, 24h or 1month",
        )),
    }
}

pub(crate) async fn history(
    State(state): State<AppState>,
    Extension(identity): Extension<Identity>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<Value>, ApiError> {
    identity.require_admin()?;
    let (seconds, interval) = range_parameters(&query.range)?;
    tokio::task::spawn_blocking(move || {
        let to = time::OffsetDateTime::now_utc().unix_timestamp();
        let from = to - seconds;
        let samples = state.telemetry.history.lock().map_err(|_| unavailable("history sampling is unavailable"))?.samples(from, to, interval);
        Ok(Json(json!({"range": query.range, "interval_seconds": interval, "from": timestamp(from)?, "to": timestamp(to)?, "samples": samples})))
    }).await.map_err(|_| unavailable("history sampling was interrupted"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_and_invalid_samples_are_not_fabricated_zero_usage() {
        assert_eq!(normalized_cpu_usage(f32::NAN), None);
        assert_eq!(normalized_cpu_usage(f32::INFINITY), None);
        assert_eq!(normalized_cpu_usage(150.0), Some(100.0));
        assert!(memory_status(0, 0).total_bytes.is_none());
        assert_eq!(
            memory_status(u64::MAX, u64::MAX - 123)
                .used_bytes
                .as_deref(),
            Some("123")
        );
    }

    #[test]
    fn real_host_sampler_starts_with_a_baseline_and_reads_memory() {
        let mut sampler = SystemSampler::new();
        let (cpu, memory) = sampler.sample();
        assert!(cpu.usage_percent.is_none());
        assert!(cpu.logical_count > 0);
        assert!(memory.total_bytes.is_some());
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        assert!(
            sampler
                .sample()
                .0
                .usage_percent
                .is_some_and(|usage| (0.0..=100.0).contains(&usage))
        );
    }

    struct MonitorBackend;
    impl Backend for MonitorBackend {
        fn call(
            &self,
            action: crate::BackendAction,
            _identity: &Identity,
            _input: Value,
        ) -> Result<Value, ApiError> {
            assert_eq!(action, crate::BackendAction::MonitorDisks);
            Ok(json!({"disks":[]}))
        }
        fn upload(
            &self,
            _: &Identity,
            _: &str,
            _: &str,
            _: &str,
            _: &Path,
        ) -> Result<Value, ApiError> {
            Err(ApiError::unsupported())
        }
        fn content(&self, _: &Identity, _: &str) -> Result<crate::Content, ApiError> {
            Err(ApiError::unsupported())
        }
    }

    #[test]
    fn unavailable_cache_falls_back_to_bounded_memory_without_losing_live_metrics() {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("cache");
        std::fs::write(&directory, b"unwritable cache path").unwrap();
        let service = Telemetry::start(Arc::new(MonitorBackend), &directory).unwrap();
        let live = service.current().unwrap();
        assert!(live.cpu.logical_count > 0);
        assert!(live.memory.total_bytes.is_some());
        let history = service.history.lock().unwrap();
        assert!(!history.is_persistent());
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        assert!(!history.samples(now - 60, now, 5).is_empty());
        assert_eq!(std::fs::read(directory).unwrap(), b"unwritable cache path");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn read_only_sysfs_cache_falls_back_to_live_memory_monitoring() {
        let service = Telemetry::start(
            Arc::new(MonitorBackend),
            Path::new("/sys/zxs-telemetry-cache-unit-test"),
        )
        .unwrap();
        assert!(!service.history.lock().unwrap().is_persistent());
        assert!(service.current().unwrap().memory.total_bytes.is_some());
    }

    #[test]
    fn supported_ranges_bound_chart_points_and_reject_other_strings() {
        for range in ["1min", "30min", "1h", "24h", "1month"] {
            let (seconds, interval) = range_parameters(range).unwrap();
            assert!(seconds / interval <= 360);
        }
        assert!(range_parameters("1year").is_err());
        assert!(range_parameters("../1h").is_err());
    }
}
