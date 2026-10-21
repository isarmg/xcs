#[test]
    fn kills_on_drop_if_specified() {
        let mut mock = Mock::new();

        {
            let guard = ChildDropGuard {
                inner: &mut mock,
                kill_on_drop: true,
            };
            drop(guard);
        }

        assert_eq!(1, mock.num_kills);
        assert_eq!(0, mock.num_polls);
    }
