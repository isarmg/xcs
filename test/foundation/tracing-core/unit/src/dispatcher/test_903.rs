#[cfg(test)]
mod test {
    #[cfg(feature = "std")]
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::{
        callsite::Callsite,
        metadata::{Kind, Level, Metadata},
        subscriber::Interest,
    };

    #[test]
    fn dispatch_is() {
        let dispatcher = Dispatch::new(NoSubscriber::default());
        assert!(dispatcher.is::<NoSubscriber>());
    }

    #[test]
    fn dispatch_downcasts() {
        let dispatcher = Dispatch::new(NoSubscriber::default());
        assert!(dispatcher.downcast_ref::<NoSubscriber>().is_some());
    }

    struct TestCallsite;
    static TEST_CALLSITE: TestCallsite = TestCallsite;
    static TEST_META: Metadata<'static> = metadata! {
        name: "test",
        target: module_path!(),
        level: Level::DEBUG,
        fields: &[],
        callsite: &TEST_CALLSITE,
        kind: Kind::EVENT
    };

    impl Callsite for TestCallsite {
        fn set_interest(&self, _: Interest) {}
        fn metadata(&self) -> &Metadata<'_> {
            &TEST_META
        }
    }

    #[test]
    #[cfg(feature = "std")]
    fn events_dont_infinite_loop() {
        // This test ensures that an event triggered within a subscriber
        // won't cause an infinite loop of events.
        struct TestSubscriber;
        impl Subscriber for TestSubscriber {
            fn enabled(&self, _: &Metadata<'_>) -> bool {
                true
            }

            fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
                span::Id::from_u64(0xAAAA)
            }

            fn record(&self, _: &span::Id, _: &span::Record<'_>) {}

            fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}

            fn event(&self, _: &Event<'_>) {
                static EVENTS: AtomicUsize = AtomicUsize::new(0);
                assert_eq!(
                    EVENTS.fetch_add(1, Ordering::Relaxed),
                    0,
                    "event method called twice!"
                );
                Event::dispatch(&TEST_META, &TEST_META.fields().value_set(&[]))
            }

            fn enter(&self, _: &span::Id) {}

            fn exit(&self, _: &span::Id) {}
        }

        with_default(&Dispatch::new(TestSubscriber), || {
            Event::dispatch(&TEST_META, &TEST_META.fields().value_set(&[]))
        })
    }

    #[test]
    #[cfg(feature = "std")]
    fn spans_dont_infinite_loop() {
        // This test ensures that a span created within a subscriber
        // won't cause an infinite loop of new spans.

        fn mk_span() {
            get_default(|current| {
                current.new_span(&span::Attributes::new(
                    &TEST_META,
                    &TEST_META.fields().value_set(&[]),
                ))
            });
        }

        struct TestSubscriber;
        impl Subscriber for TestSubscriber {
            fn enabled(&self, _: &Metadata<'_>) -> bool {
                true
            }

            fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
                static NEW_SPANS: AtomicUsize = AtomicUsize::new(0);
                assert_eq!(
                    NEW_SPANS.fetch_add(1, Ordering::Relaxed),
                    0,
                    "new_span method called twice!"
                );
                mk_span();
                span::Id::from_u64(0xAAAA)
            }

            fn record(&self, _: &span::Id, _: &span::Record<'_>) {}

            fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}

            fn event(&self, _: &Event<'_>) {}

            fn enter(&self, _: &span::Id) {}

            fn exit(&self, _: &span::Id) {}
        }

        with_default(&Dispatch::new(TestSubscriber), mk_span)
    }

    #[test]
    fn default_no_subscriber() {
        let default_dispatcher = Dispatch::default();
        assert!(default_dispatcher.is::<NoSubscriber>());
    }

    #[cfg(feature = "std")]
    #[test]
    fn default_dispatch() {
        struct TestSubscriber;
        impl Subscriber for TestSubscriber {
            fn enabled(&self, _: &Metadata<'_>) -> bool {
                true
            }

            fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
                span::Id::from_u64(0xAAAA)
            }

            fn record(&self, _: &span::Id, _: &span::Record<'_>) {}

            fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}

            fn event(&self, _: &Event<'_>) {}

            fn enter(&self, _: &span::Id) {}

            fn exit(&self, _: &span::Id) {}
        }
        let guard = set_default(&Dispatch::new(TestSubscriber));
        let default_dispatcher = Dispatch::default();
        assert!(default_dispatcher.is::<TestSubscriber>());

        drop(guard);
        let default_dispatcher = Dispatch::default();
        assert!(default_dispatcher.is::<NoSubscriber>());
    }
}
