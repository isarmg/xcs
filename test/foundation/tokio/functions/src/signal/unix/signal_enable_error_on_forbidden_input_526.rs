#[test]
    fn signal_enable_error_on_forbidden_input() {
        let inputs = signal_hook_registry::FORBIDDEN;

        for &input in inputs {
            assert_eq!(
                signal_enable(SignalKind::from_raw(input), &Handle::default())
                    .unwrap_err()
                    .kind(),
                ErrorKind::Other,
            );
        }
    }
