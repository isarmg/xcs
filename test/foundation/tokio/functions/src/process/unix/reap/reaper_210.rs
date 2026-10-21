#[test]
    fn reaper() {
        let exit = ExitStatus::from_raw(0);
        let mock = MockWait::new(exit, 3);
        let mut grim = Reaper::new(
            mock,
            MockQueue::new(),
            MockStream::new(vec![None, Some(()), None, None, None]),
        );

        let waker = futures::task::noop_waker();
        let mut context = Context::from_waker(&waker);

        // Not yet exited, interest registered
        assert!(grim.poll_unpin(&mut context).is_pending());
        assert_eq!(1, grim.signal.total_polls);
        assert_eq!(1, grim.total_waits);
        assert!(grim.orphan_queue.all_enqueued.borrow().is_empty());

        // Not yet exited, couldn't register interest the first time
        // but managed to register interest the second time around
        assert!(grim.poll_unpin(&mut context).is_pending());
        assert_eq!(3, grim.signal.total_polls);
        assert_eq!(3, grim.total_waits);
        assert!(grim.orphan_queue.all_enqueued.borrow().is_empty());

        // Exited
        if let Poll::Ready(r) = grim.poll_unpin(&mut context) {
            assert!(r.is_ok());
            let exit_code = r.unwrap();
            assert_eq!(exit_code, exit);
        } else {
            unreachable!();
        }
        assert_eq!(4, grim.signal.total_polls);
        assert_eq!(4, grim.total_waits);
        assert!(grim.orphan_queue.all_enqueued.borrow().is_empty());
    }
