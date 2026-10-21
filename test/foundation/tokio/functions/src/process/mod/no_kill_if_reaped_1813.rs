#[test]
    fn no_kill_if_reaped() {
        let mut mock_pending = Mock::with_result(Poll::Pending);
        let mut mock_reaped = Mock::with_result(Poll::Ready(Ok(())));
        let mut mock_err = Mock::with_result(Poll::Ready(Err(())));

        let waker = futures::task::noop_waker();
        let mut context = Context::from_waker(&waker);
        {
            let mut guard = ChildDropGuard {
                inner: &mut mock_pending,
                kill_on_drop: true,
            };
            let _ = guard.poll_unpin(&mut context);

            let mut guard = ChildDropGuard {
                inner: &mut mock_reaped,
                kill_on_drop: true,
            };
            let _ = guard.poll_unpin(&mut context);

            let mut guard = ChildDropGuard {
                inner: &mut mock_err,
                kill_on_drop: true,
            };
            let _ = guard.poll_unpin(&mut context);
        }

        assert_eq!(1, mock_pending.num_kills);
        assert_eq!(1, mock_pending.num_polls);

        assert_eq!(0, mock_reaped.num_kills);
        assert_eq!(1, mock_reaped.num_polls);

        assert_eq!(1, mock_err.num_kills);
        assert_eq!(1, mock_err.num_polls);
    }
