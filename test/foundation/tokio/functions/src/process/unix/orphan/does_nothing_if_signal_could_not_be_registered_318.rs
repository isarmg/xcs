#[test]
    fn does_nothing_if_signal_could_not_be_registered() {
        let handle = SignalHandle::default();

        let orphanage = OrphanQueueImpl::new();
        assert!(orphanage.sigchild.lock().is_none());

        let orphan = MockWait::new(2);
        let waits = orphan.total_waits.clone();
        orphanage.push_orphan(orphan);

        // Signal handler has "gone away", nothing to register or reap
        orphanage.reap_orphans(&handle);
        assert!(orphanage.sigchild.lock().is_none());
        assert_eq!(waits.get(), 0);
    }
