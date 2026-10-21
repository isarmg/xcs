#[cfg(test)]
mod test {
    use crate::*;
    use std::str::FromStr;

    // In case `Process::updated` is misused, `System::refresh_processes` might remove them
    // so this test ensures that it doesn't happen.
    #[test]
    fn check_refresh_process_update() {
        if !IS_SUPPORTED_SYSTEM {
            return;
        }
        let mut s = System::new_all();
        let total = s.processes().len() as isize;
        s.refresh_processes(ProcessesToUpdate::All, false);
        let new_total = s.processes().len() as isize;
        // There should be almost no difference in the processes count.
        assert!(
            (new_total - total).abs() <= 5,
            "{} <= 5",
            (new_total - total).abs()
        );
    }

    #[test]
    fn check_cpu_arch() {
        assert!(!System::cpu_arch().is_empty());
    }

    // Ensure that the CPUs frequency isn't retrieved until we ask for it.
    #[test]
    fn check_cpu_frequency() {
        if !IS_SUPPORTED_SYSTEM {
            return;
        }
        let mut s = System::new();
        s.refresh_processes(ProcessesToUpdate::All, false);
        for proc_ in s.cpus() {
            assert_eq!(proc_.frequency(), 0);
        }
        s.refresh_cpu_usage();
        for proc_ in s.cpus() {
            assert_eq!(proc_.frequency(), 0);
        }
        // In a VM, it'll fail.
        if std::env::var("APPLE_CI").is_ok()
            || std::env::var("FREEBSD_CI").is_ok()
            || std::env::var("NETBSD_CI").is_ok()
        {
            return;
        }
        s.refresh_cpu_specifics(CpuRefreshKind::everything());
        for proc_ in s.cpus() {
            assert_ne!(proc_.frequency(), 0);
        }
    }

    #[test]
    fn check_process_memory_usage() {
        let mut s = System::new();
        s.refresh_specifics(RefreshKind::everything());

        if IS_SUPPORTED_SYSTEM {
            // No process should have 0 as memory usage.
            #[cfg(not(feature = "apple-sandbox"))]
            assert!(!s.processes().iter().all(|(_, proc_)| proc_.memory() == 0));
        } else {
            // There should be no process, but if there is one, its memory usage should be 0.
            assert!(s.processes().iter().all(|(_, proc_)| proc_.memory() == 0));
        }
    }

    #[test]
    fn check_system_implemented_traits() {
        fn check<T: Sized + std::fmt::Debug + Default + Send + Sync>(_: T) {}

        check(System::new());
    }

    #[test]
    fn check_memory_usage() {
        let mut s = System::new();

        assert_eq!(s.total_memory(), 0);
        assert_eq!(s.free_memory(), 0);
        assert_eq!(s.available_memory(), 0);
        assert_eq!(s.used_memory(), 0);
        assert_eq!(s.total_swap(), 0);
        assert_eq!(s.free_swap(), 0);
        assert_eq!(s.used_swap(), 0);

        s.refresh_memory();
        if IS_SUPPORTED_SYSTEM {
            assert!(s.total_memory() > 0);
            assert!(s.used_memory() > 0);
            if s.total_swap() > 0 {
                // I think it's pretty safe to assume that there is still some swap left...
                assert!(s.free_swap() > 0);
            }
        } else {
            assert_eq!(s.total_memory(), 0);
            assert_eq!(s.used_memory(), 0);
            assert_eq!(s.total_swap(), 0);
            assert_eq!(s.free_swap(), 0);
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn check_processes_cpu_usage() {
        if !IS_SUPPORTED_SYSTEM {
            return;
        }
        let mut s = System::new();

        s.refresh_processes(ProcessesToUpdate::All, false);
        // All CPU usage will start at zero until the second refresh
        assert!(
            s.processes()
                .iter()
                .all(|(_, proc_)| proc_.cpu_usage() == 0.0)
        );

        // Wait a bit to update CPU usage values
        std::thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL);
        s.refresh_processes(ProcessesToUpdate::All, true);
        assert!(
            s.processes()
                .iter()
                .all(|(_, proc_)| proc_.cpu_usage() >= 0.0
                    && proc_.cpu_usage() <= (s.cpus().len() as f32) * 100.0)
        );
        assert!(
            s.processes()
                .iter()
                .any(|(_, proc_)| proc_.cpu_usage() > 0.0)
        );
    }

    #[test]
    fn check_cpu_usage() {
        if !IS_SUPPORTED_SYSTEM {
            return;
        }
        let mut s = System::new();
        for _ in 0..10 {
            s.refresh_cpu_usage();
            // Wait a bit to update CPU usage values
            std::thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL);
            if s.cpus().iter().any(|c| c.cpu_usage() > 0.0) {
                // All good!
                return;
            }
        }
        panic!("CPU usage is always zero...");
    }

    #[test]
    fn check_system_info() {
        // We don't want to test on unsupported systems.
        if IS_SUPPORTED_SYSTEM {
            assert!(
                !System::name()
                    .expect("Failed to get system name")
                    .is_empty()
            );

            assert!(
                !System::kernel_version()
                    .expect("Failed to get kernel version")
                    .is_empty()
            );

            assert!(
                !System::os_version()
                    .expect("Failed to get os version")
                    .is_empty()
            );

            assert!(
                !System::long_os_version()
                    .expect("Failed to get long OS version")
                    .is_empty()
            );
        }

        assert!(!System::distribution_id().is_empty());
    }

    #[test]
    fn check_host_name() {
        // We don't want to test on unsupported systems.
        if IS_SUPPORTED_SYSTEM {
            assert!(System::host_name().is_some());
        }
    }

    #[test]
    fn check_refresh_process_return_value() {
        // We don't want to test on unsupported systems.
        if IS_SUPPORTED_SYSTEM {
            let _pid = get_current_pid().expect("Failed to get current PID");

            #[cfg(not(feature = "apple-sandbox"))]
            {
                let mut s = System::new();
                // First check what happens in case the process isn't already in our process list.
                assert_eq!(
                    s.refresh_processes(ProcessesToUpdate::Some(&[_pid]), true),
                    1
                );
                // Then check that it still returns 1 if the process is already in our process list.
                assert_eq!(
                    s.refresh_processes(ProcessesToUpdate::Some(&[_pid]), true),
                    1
                );
            }
        }
    }

    #[test]
    fn check_cpus_number() {
        let mut s = System::new();

        // This information isn't retrieved by default.
        assert!(s.cpus().is_empty());
        if IS_SUPPORTED_SYSTEM {
            // The physical cores count is recomputed every time the function is called, so the
            // information must be relevant even with nothing initialized.
            let physical_cores_count =
                System::physical_core_count().expect("failed to get number of physical cores");

            s.refresh_cpu_usage();
            // The cpus shouldn't be empty anymore.
            assert!(!s.cpus().is_empty());

            // In case we are running inside a VM, it's possible to not have a physical core, only
            // logical ones, which is why we don't test `physical_cores_count > 0`.
            let physical_cores_count2 =
                System::physical_core_count().expect("failed to get number of physical cores");
            assert!(physical_cores_count2 <= s.cpus().len());
            assert_eq!(physical_cores_count, physical_cores_count2);
        } else {
            assert_eq!(System::physical_core_count(), None);
        }
        assert!(System::physical_core_count().unwrap_or(0) <= s.cpus().len());
    }

    // This test only exists to ensure that the `Display` and `Debug` traits are implemented on the
    // `ProcessStatus` enum on all targets.
    #[test]
    fn check_display_impl_process_status() {
        println!("{} {:?}", ProcessStatus::Parked, ProcessStatus::Idle);
    }

    #[test]
    #[allow(clippy::unnecessary_fallible_conversions)]
    fn check_pid_from_impls() {
        assert!(crate::Pid::try_from(0usize).is_ok());
        // If it doesn't panic, it's fine.
        let _ = crate::Pid::from(0);
        assert!(crate::Pid::from_str("0").is_ok());
    }

    #[test]
    #[allow(clippy::const_is_empty)]
    fn check_nb_supported_signals() {
        if IS_SUPPORTED_SYSTEM {
            assert!(
                !SUPPORTED_SIGNALS.is_empty(),
                "SUPPORTED_SIGNALS shouldn't be empty on supported systems!"
            );
        } else {
            assert!(
                SUPPORTED_SIGNALS.is_empty(),
                "SUPPORTED_SIGNALS should be empty on not support systems!"
            );
        }
    }
}
