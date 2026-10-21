#[test]
    fn drain_attempts_a_single_reap_of_all_queued_orphans() {
        let first_orphan = MockWait::new(0);
        let second_orphan = MockWait::new(1);
        let third_orphan = MockWait::new(2);
        let fourth_orphan = MockWait::with_err();

        let first_waits = first_orphan.total_waits.clone();
        let second_waits = second_orphan.total_waits.clone();
        let third_waits = third_orphan.total_waits.clone();
        let fourth_waits = fourth_orphan.total_waits.clone();

        let orphanage = OrphanQueueImpl::new();
        orphanage.push_orphan(first_orphan);
        orphanage.push_orphan(third_orphan);
        orphanage.push_orphan(second_orphan);
        orphanage.push_orphan(fourth_orphan);

        assert_eq!(orphanage.len(), 4);

        drain_orphan_queue(orphanage.queue.lock());
        assert_eq!(orphanage.len(), 2);
        assert_eq!(first_waits.get(), 1);
        assert_eq!(second_waits.get(), 1);
        assert_eq!(third_waits.get(), 1);
        assert_eq!(fourth_waits.get(), 1);

        drain_orphan_queue(orphanage.queue.lock());
        assert_eq!(orphanage.len(), 1);
        assert_eq!(first_waits.get(), 1);
        assert_eq!(second_waits.get(), 2);
        assert_eq!(third_waits.get(), 2);
        assert_eq!(fourth_waits.get(), 1);

        drain_orphan_queue(orphanage.queue.lock());
        assert_eq!(orphanage.len(), 0);
        assert_eq!(first_waits.get(), 1);
        assert_eq!(second_waits.get(), 2);
        assert_eq!(third_waits.get(), 3);
        assert_eq!(fourth_waits.get(), 1);

        // Safe to reap when empty
        drain_orphan_queue(orphanage.queue.lock());
    }
