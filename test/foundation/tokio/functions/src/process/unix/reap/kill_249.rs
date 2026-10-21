#[test]
    fn kill() {
        let exit = ExitStatus::from_raw(0);
        let mut grim = Reaper::new(
            MockWait::new(exit, 0),
            MockQueue::new(),
            MockStream::new(vec![None]),
        );

        grim.kill().unwrap();
        assert_eq!(1, grim.total_kills);
        assert!(grim.orphan_queue.all_enqueued.borrow().is_empty());
    }
