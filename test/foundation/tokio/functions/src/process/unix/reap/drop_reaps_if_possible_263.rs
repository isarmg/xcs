#[test]
    fn drop_reaps_if_possible() {
        let exit = ExitStatus::from_raw(0);
        let mut mock = MockWait::new(exit, 0);

        {
            let queue = MockQueue::new();

            let grim = Reaper::new(&mut mock, &queue, MockStream::new(vec![]));

            drop(grim);

            assert!(queue.all_enqueued.borrow().is_empty());
        }

        assert_eq!(1, mock.total_waits);
        assert_eq!(0, mock.total_kills);
    }
