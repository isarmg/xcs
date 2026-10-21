#[cfg_attr(miri, ignore)] // No `sigaction` on Miri
    #[test]
    fn does_not_register_signal_if_queue_empty() {
        let (io_driver, io_handle) = IoDriver::new(1024).unwrap();
        let signal_driver = SignalDriver::new(io_driver, &io_handle).unwrap();
        let handle = signal_driver.handle();

        let orphanage = OrphanQueueImpl::new();
        assert!(orphanage.sigchild.lock().is_none()); // Sanity

        // No register when queue empty
        orphanage.reap_orphans(&handle);
        assert!(orphanage.sigchild.lock().is_none());

        let orphan = MockWait::new(2);
        let waits = orphan.total_waits.clone();
        orphanage.push_orphan(orphan);

        orphanage.reap_orphans(&handle);
        assert!(orphanage.sigchild.lock().is_some());
        assert_eq!(waits.get(), 1); // Eager reap when registering listener
    }
