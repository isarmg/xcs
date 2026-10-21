#[test]
    fn no_reap_if_no_signal_received() {
        let (tx, rx) = watch::channel(());

        let handle = SignalHandle::default();

        let orphanage = OrphanQueueImpl::new();
        *orphanage.sigchild.lock() = Some(rx);

        let orphan = MockWait::new(2);
        let waits = orphan.total_waits.clone();
        orphanage.push_orphan(orphan);

        orphanage.reap_orphans(&handle);
        assert_eq!(waits.get(), 0);

        orphanage.reap_orphans(&handle);
        assert_eq!(waits.get(), 0);

        tx.send(()).unwrap();
        orphanage.reap_orphans(&handle);
        assert_eq!(waits.get(), 1);
    }
