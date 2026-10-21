#[test]
    fn no_kill_on_drop_by_default() {
        let mut mock = Mock::new();

        {
            let guard = ChildDropGuard {
                inner: &mut mock,
                kill_on_drop: false,
            };
            drop(guard);
        }

        assert_eq!(0, mock.num_kills);
        assert_eq!(0, mock.num_polls);
    }
