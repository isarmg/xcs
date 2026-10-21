#[test]
    fn record_invalid_event_does_nothing() {
        let registry = Registry::new(vec![EventInfo::default()]);
        registry.record_event(1302);
    }
