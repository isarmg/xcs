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
        _: crate::BackendAction,
        _identity: &Identity,
        _input: Value,
    ) -> Result<Value, ApiError> {
        panic!("private monitoring must not forge a user identity")
    }
    fn observe_disks(&self, _: &crate::InternalDiskObservation) -> Result<Value, ApiError> {
        Ok(json!({"disks":[]}))
    }
    fn upload(&self, _: &Identity, _: &str, _: &str, _: &str, _: &Path) -> Result<Value, ApiError> {
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

#[test]
fn monitoring_rechecks_current_administrator_instead_of_captured_role() {
    let root = tempfile::tempdir().unwrap();
    let mut auth =
        crate::auth::AuthStore::initialize(root.path(), "admin", "monitor-test-password").unwrap();
    auth.upsert_account(
        "monitor",
        Some("monitor-test-password"),
        true,
        Default::default(),
        false,
    )
    .unwrap();
    let captured = auth.identity_for_principal("monitor").unwrap();
    let auth = Mutex::new(auth);
    assert!(require_current_administrator(&auth, &captured).is_ok());
    for disabled in [false, true] {
        auth.lock()
            .unwrap()
            .upsert_account("monitor", None, false, Default::default(), disabled)
            .unwrap();
        assert_eq!(
            require_current_administrator(&auth, &captured)
                .unwrap_err()
                .code,
            if disabled {
                "UNAUTHORIZED"
            } else {
                "FORBIDDEN"
            }
        );
    }
}
