#[test]
    fn from_c_int() {
        assert_eq!(SignalKind::from(2), SignalKind::interrupt());
    }
