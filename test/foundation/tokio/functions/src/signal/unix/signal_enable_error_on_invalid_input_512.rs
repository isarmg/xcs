#[test]
    fn signal_enable_error_on_invalid_input() {
        let inputs = [-1, 0];

        for input in inputs {
            assert_eq!(
                signal_enable(SignalKind::from_raw(input), &Handle::default())
                    .unwrap_err()
                    .kind(),
                ErrorKind::Other,
            );
        }
    }
