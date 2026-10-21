#[test]
    fn no_kill_if_already_killed() {
        let mut mock = Mock::new();

        {
            let mut guard = ChildDropGuard {
                inner: &mut mock,
                kill_on_drop: true,
            };
            let _ = guard.kill();
            drop(guard);
        }

        assert_eq!(1, mock.num_kills);
        assert_eq!(0, mock.num_polls);
    }
