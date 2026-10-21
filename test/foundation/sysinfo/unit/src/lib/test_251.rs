#[cfg(test)]
mod test {
    use crate::*;

    #[cfg(feature = "unknown-ci")]
    #[test]
    fn check_unknown_ci_feature() {
        assert!(!IS_SUPPORTED_SYSTEM);
    }

    // If this test doesn't compile, it means the current OS doesn't implement them correctly.
    #[test]
    fn check_macro_types() {
        fn check_is_supported(_: bool) {}

        check_is_supported(IS_SUPPORTED_SYSTEM);
    }

    // If this test doesn't compile, it means the current OS doesn't implement them correctly.
    #[cfg(feature = "system")]
    #[test]
    fn check_macro_types2() {
        fn check_supported_signals(_: &'static [Signal]) {}
        fn check_minimum_cpu_update_interval(_: std::time::Duration) {}

        check_supported_signals(SUPPORTED_SIGNALS);
        check_minimum_cpu_update_interval(MINIMUM_CPU_UPDATE_INTERVAL);
    }

    #[cfg(feature = "user")]
    #[test]
    fn check_uid_gid() {
        let mut users = Users::new();
        assert!(users.list().is_empty());
        users.refresh();
        let user_list = users.list();
        assert!(user_list.len() >= MIN_USERS);

        if IS_SUPPORTED_SYSTEM {
            #[cfg(not(target_os = "windows"))]
            {
                let user = user_list
                    .iter()
                    .find(|u| u.name() == "root")
                    .expect("no root user");
                assert_eq!(**user.id(), 0);
                assert_eq!(*user.group_id(), 0);
                if let Some(user) = users.iter().find(|u| *u.group_id() > 0) {
                    assert!(**user.id() > 0);
                    assert!(*user.group_id() > 0);
                }
                assert!(user_list.iter().filter(|u| **u.id() > 0).count() > 0);
            }

            #[cfg(feature = "system")]
            {
                // And now check that our `get_user_by_id` method works.
                let s =
                    System::new_with_specifics(RefreshKind::nothing().with_processes(
                        ProcessRefreshKind::nothing().with_user(UpdateKind::Always),
                    ));
                assert!(
                    s.processes()
                        .values()
                        .filter_map(|p| p.user_id())
                        .any(|uid| users.get_user_by_id(uid).is_some())
                );
            }
        }
    }

    #[cfg(all(feature = "system", feature = "user"))]
    #[test]
    fn check_all_process_uids_resolvable() {
        // On linux, some user IDs don't have an associated user (no idea why though).
        // If `getent` doesn't find them, we can assume it's a dark secret from the linux land.
        if IS_SUPPORTED_SYSTEM && cfg!(not(target_os = "linux")) {
            let s = System::new_with_specifics(
                RefreshKind::nothing()
                    .with_processes(ProcessRefreshKind::nothing().with_user(UpdateKind::Always)),
            );
            let users = Users::new_with_refreshed_list();

            // For every process where we can get a user ID, we should also be able
            // to find that user ID in the global user list
            for process in s.processes().values() {
                if let Some(uid) = process.user_id() {
                    assert!(users.get_user_by_id(uid).is_some(), "No UID {uid:?} found");
                }
            }
        }
    }

    #[test]
    fn ensure_is_supported_is_set_correctly() {
        if MIN_USERS > 0 {
            assert!(IS_SUPPORTED_SYSTEM);
        } else {
            assert!(!IS_SUPPORTED_SYSTEM);
        }
    }

    // If it doesn't compile, it means types don't implement expected traits.
    #[cfg(any(
        feature = "system",
        feature = "disk",
        feature = "component",
        feature = "user",
        feature = "network"
    ))]
    #[test]
    fn test_send_and_sync() {
        #[allow(dead_code)]
        trait HasSendAndSync: Send + Sync {}

        // Structs
        impl HasSendAndSync for CGroupLimits {}
        impl HasSendAndSync for Component {}
        impl HasSendAndSync for Components {}
        impl HasSendAndSync for Cpu {}
        impl HasSendAndSync for CpuRefreshKind {}
        impl HasSendAndSync for Disk {}
        impl HasSendAndSync for Disks {}
        impl HasSendAndSync for DiskRefreshKind {}
        impl HasSendAndSync for DiskUsage {}
        impl HasSendAndSync for Gid {}
        impl HasSendAndSync for Group {}
        impl HasSendAndSync for Groups {}
        impl HasSendAndSync for IpNetwork {}
        impl HasSendAndSync for LoadAvg {}
        impl HasSendAndSync for MacAddr {}
        impl HasSendAndSync for MemoryRefreshKind {}
        impl HasSendAndSync for NetworkData {}
        impl HasSendAndSync for Networks {}
        impl HasSendAndSync for Pid {}
        impl HasSendAndSync for Process {}
        impl HasSendAndSync for ProcessRefreshKind {}
        impl HasSendAndSync for Product {}
        impl HasSendAndSync for RefreshKind {}
        impl HasSendAndSync for System {}
        impl HasSendAndSync for Uid {}
        impl HasSendAndSync for User {}
        impl HasSendAndSync for Users {}

        // Enums
        impl HasSendAndSync for DiskKind {}
        impl HasSendAndSync for IpNetworkFromStrError {}
        impl HasSendAndSync for KillError {}
        impl HasSendAndSync for MacAddrFromStrError {}
        impl HasSendAndSync for ProcessStatus {}
        impl HasSendAndSync for ProcessesToUpdate<'_> {}
        impl HasSendAndSync for Signal {}
        impl HasSendAndSync for ThreadKind {}
        impl HasSendAndSync for UpdateKind {}
    }
}
