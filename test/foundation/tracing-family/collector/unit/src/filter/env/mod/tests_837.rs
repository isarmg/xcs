#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;
    use std::println;
    use tracing::primitives::field::FieldSet;
    use tracing::primitives::*;

    struct NoSubscriber;
    impl Subscriber for NoSubscriber {
        #[inline]
        fn register_callsite(&self, _: &'static Metadata<'static>) -> subscriber::Interest {
            subscriber::Interest::always()
        }
        fn new_span(&self, _: &span::Attributes<'_>) -> span::Id {
            span::Id::from_u64(0xDEAD)
        }
        fn event(&self, _event: &Event<'_>) {}
        fn record(&self, _span: &span::Id, _values: &span::Record<'_>) {}
        fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id) {}

        #[inline]
        fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
            true
        }
        fn enter(&self, _span: &span::Id) {}
        fn exit(&self, _span: &span::Id) {}
    }

    struct Cs;
    impl Callsite for Cs {
        fn set_interest(&self, _interest: Interest) {}
        fn metadata(&self) -> &Metadata<'_> {
            unimplemented!()
        }
    }

    #[test]
    fn callsite_enabled_no_span_directive() {
        let filter = EnvFilter::new("app=debug").with_subscriber(NoSubscriber);
        static META: &Metadata<'static> = &Metadata::new(
            "mySpan",
            "app",
            Level::TRACE,
            None,
            None,
            None,
            FieldSet::new(&[], identify_callsite!(&Cs)),
            Kind::SPAN,
        );

        let interest = filter.register_callsite(META);
        assert!(interest.is_never());
    }

    #[test]
    fn callsite_off() {
        let filter = EnvFilter::new("app=off").with_subscriber(NoSubscriber);
        static META: &Metadata<'static> = &Metadata::new(
            "mySpan",
            "app",
            Level::ERROR,
            None,
            None,
            None,
            FieldSet::new(&[], identify_callsite!(&Cs)),
            Kind::SPAN,
        );

        let interest = filter.register_callsite(META);
        assert!(interest.is_never());
    }

    #[test]
    fn callsite_enabled_includes_span_directive() {
        let filter = EnvFilter::new("app[mySpan]=debug").with_subscriber(NoSubscriber);
        static META: &Metadata<'static> = &Metadata::new(
            "mySpan",
            "app",
            Level::TRACE,
            None,
            None,
            None,
            FieldSet::new(&[], identify_callsite!(&Cs)),
            Kind::SPAN,
        );

        let interest = filter.register_callsite(META);
        assert!(interest.is_always());
    }

    #[test]
    fn callsite_enabled_includes_span_directive_field() {
        let filter =
            EnvFilter::new("app[mySpan{field=\"value\"}]=debug").with_subscriber(NoSubscriber);
        static META: &Metadata<'static> = &Metadata::new(
            "mySpan",
            "app",
            Level::TRACE,
            None,
            None,
            None,
            FieldSet::new(&["field"], identify_callsite!(&Cs)),
            Kind::SPAN,
        );

        let interest = filter.register_callsite(META);
        assert!(interest.is_always());
    }

    #[test]
    fn callsite_enabled_includes_span_directive_multiple_fields() {
        let filter = EnvFilter::new("app[mySpan{field=\"value\",field2=2}]=debug")
            .with_subscriber(NoSubscriber);
        static META: &Metadata<'static> = &Metadata::new(
            "mySpan",
            "app",
            Level::TRACE,
            None,
            None,
            None,
            FieldSet::new(&["field"], identify_callsite!(&Cs)),
            Kind::SPAN,
        );

        let interest = filter.register_callsite(META);
        assert!(interest.is_never());
    }

    #[test]
    fn roundtrip() {
        let f1: EnvFilter =
            "[span1{foo=1}]=error,[span2{bar=2 baz=false}],crate2[{quux=\"quuux\"}]=debug"
                .parse()
                .unwrap();
        let f2: EnvFilter = format!("{}", f1).parse().unwrap();
        assert_eq!(f1.statics, f2.statics);
        assert_eq!(f1.dynamics, f2.dynamics);
    }

    #[test]
    fn size_of_filters() {
        fn print_sz(s: &str) {
            let filter = s.parse::<EnvFilter>().expect("filter should parse");
            println!(
                "size_of_val({:?})\n -> {}B",
                s,
                std::mem::size_of_val(&filter)
            );
        }

        print_sz("info");

        print_sz("foo=debug");

        print_sz(
            "crate1::mod1=error,crate1::mod2=warn,crate1::mod2::mod3=info,\
            crate2=debug,crate3=trace,crate3::mod2::mod1=off",
        );

        print_sz("[span1{foo=1}]=error,[span2{bar=2 baz=false}],crate2[{quux=\"quuux\"}]=debug");

        print_sz(
            "crate1::mod1=error,crate1::mod2=warn,crate1::mod2::mod3=info,\
            crate2=debug,crate3=trace,crate3::mod2::mod1=off,[span1{foo=1}]=error,\
            [span2{bar=2 baz=false}],crate2[{quux=\"quuux\"}]=debug",
        );
    }

    #[test]
    fn parse_empty_string() {
        // There is no corresponding test for [`Builder::parse_lossy`] as failed
        // parsing does not produce any observable side effects. If this test fails
        // check that [`Builder::parse_lossy`] is behaving correctly as well.
        assert!(EnvFilter::builder().parse("").is_ok());
    }
}
