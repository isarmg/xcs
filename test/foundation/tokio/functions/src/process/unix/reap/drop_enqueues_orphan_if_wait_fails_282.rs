#[test]
    fn drop_enqueues_orphan_if_wait_fails() {
        let exit = ExitStatus::from_raw(0);
        let mut mock = MockWait::new(exit, 2);

        {
            let queue = MockQueue::<&mut MockWait>::new();
            let grim = Reaper::new(&mut mock, &queue, MockStream::new(vec![]));
            drop(grim);

            assert_eq!(1, queue.all_enqueued.borrow().len());
        }

        assert_eq!(1, mock.total_waits);
        assert_eq!(0, mock.total_kills);
    }
