#[test]
    fn broadcast_returns_if_at_least_one_event_fired() {
        let registry = Registry::new(vec![EventInfo::default(), EventInfo::default()]);

        registry.record_event(0);
        assert!(!registry.broadcast());

        let first = registry.register_listener(0);
        let second = registry.register_listener(1);

        registry.record_event(0);
        assert!(registry.broadcast());

        drop(first);
        registry.record_event(0);
        assert!(!registry.broadcast());

        drop(second);
    }
