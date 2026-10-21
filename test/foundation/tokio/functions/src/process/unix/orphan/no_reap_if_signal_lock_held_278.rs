#[test]
    fn no_reap_if_signal_lock_held() {
        let handle = SignalHandle::default();

        let orphanage = OrphanQueueImpl::new();
        let signal_guard = orphanage.sigchild.lock();

        let orphan = MockWait::new(2);
        let waits = orphan.total_waits.clone();
        orphanage.push_orphan(orphan);

        orphanage.reap_orphans(&handle);
        assert_eq!(waits.get(), 0);

        drop(signal_guard);
    }
