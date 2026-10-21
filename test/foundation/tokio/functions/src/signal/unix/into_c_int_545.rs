#[test]
    fn into_c_int() {
        let value: std::os::raw::c_int = SignalKind::interrupt().into();
        assert_eq!(value, libc::SIGINT as _);
    }
