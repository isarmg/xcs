#[cfg(test)]
mod test {
    use super::*;
    use crate::fmt::{
        self,
        format::{self, test::MockTime, Format},
        layer::Layer as _,
        test::{MockMakeWriter, MockWriter},
        time,
    };
    use crate::Registry;
    use alloc::{string::ToString, vec, vec::Vec};
    use format::FmtSpan;
    use regex::Regex;
    use tracing::subscriber::with_default;
    use tracing_core::dispatcher::Dispatch;

    #[test]
    fn impls() {
        let f = Format::default().with_timer(time::Uptime::default());
        let fmt = fmt::Layer::default().event_format(f);
        let subscriber = fmt.with_subscriber(Registry::default());
        let _dispatch = Dispatch::new(subscriber);

        let f = format::Format::default();
        let fmt = fmt::Layer::default().event_format(f);
        let subscriber = fmt.with_subscriber(Registry::default());
        let _dispatch = Dispatch::new(subscriber);

        let f = format::Format::default().compact();
        let fmt = fmt::Layer::default().event_format(f);
        let subscriber = fmt.with_subscriber(Registry::default());
        let _dispatch = Dispatch::new(subscriber);
    }

    #[test]
    fn fmt_layer_downcasts() {
        let f = format::Format::default();
        let fmt = fmt::Layer::default().event_format(f);
        let subscriber = fmt.with_subscriber(Registry::default());

        let dispatch = Dispatch::new(subscriber);
        assert!(dispatch.downcast_ref::<fmt::Layer<Registry>>().is_some());
    }

    #[test]
    fn fmt_layer_downcasts_to_parts() {
        let f = format::Format::default();
        let fmt = fmt::Layer::default().event_format(f);
        let subscriber = fmt.with_subscriber(Registry::default());
        let dispatch = Dispatch::new(subscriber);
        assert!(dispatch.downcast_ref::<format::DefaultFields>().is_some());
        assert!(dispatch.downcast_ref::<format::Format>().is_some())
    }

    #[test]
    fn is_lookup_span() {
        fn assert_lookup_span<T: for<'a> crate::registry::LookupSpan<'a>>(_: T) {}
        let fmt = fmt::Layer::default();
        let subscriber = fmt.with_subscriber(Registry::default());
        assert_lookup_span(subscriber)
    }

    fn sanitize_timings(s: String) -> String {
        let re = Regex::new("time\\.(idle|busy)=([0-9.]+)[mµn]s").unwrap();
        re.replace_all(s.as_str(), "timing").to_string()
    }

    #[test]
    fn format_error_print_to_stderr() {
        struct AlwaysError;

        impl std::fmt::Debug for AlwaysError {
            fn fmt(&self, _f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                Err(std::fmt::Error)
            }
        }

        let make_writer = MockMakeWriter::default();
        let subscriber = crate::fmt::Subscriber::builder()
            .with_writer(make_writer.clone())
            .with_level(false)
            .with_ansi(false)
            .with_timer(MockTime)
            .finish();

        with_default(subscriber, || {
            tracing::info!(?AlwaysError);
        });
        let actual = sanitize_timings(make_writer.get_string());

        // Only assert the start because the line number and callsite may change.
        let expected = concat!(
            "Unable to format the following event. Name: event ",
            file!(),
            ":"
        );
        assert!(
            actual.as_str().starts_with(expected),
            "\nactual = {}\nshould start with expected = {}\n",
            actual,
            expected
        );
    }

    #[test]
    fn format_error_ignore_if_log_internal_errors_is_false() {
        struct AlwaysError;

        impl std::fmt::Debug for AlwaysError {
            fn fmt(&self, _f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                Err(std::fmt::Error)
            }
        }

        let make_writer = MockMakeWriter::default();
        let subscriber = crate::fmt::Subscriber::builder()
            .with_writer(make_writer.clone())
            .with_level(false)
            .with_ansi(false)
            .with_timer(MockTime)
            .log_internal_errors(false)
            .finish();

        with_default(subscriber, || {
            tracing::info!(?AlwaysError);
        });
        let actual = sanitize_timings(make_writer.get_string());
        assert_eq!("", actual.as_str());
    }

    #[test]
    fn synthesize_span_none() {
        let make_writer = MockMakeWriter::default();
        let subscriber = crate::fmt::Subscriber::builder()
            .with_writer(make_writer.clone())
            .with_level(false)
            .with_ansi(false)
            .with_timer(MockTime)
            // check that FmtSpan::NONE is the default
            .finish();

        with_default(subscriber, || {
            let span1 = tracing::info_span!("span1", x = 42);
            let _e = span1.enter();
        });
        let actual = sanitize_timings(make_writer.get_string());
        assert_eq!("", actual.as_str());
    }

    #[test]
    fn synthesize_span_active() {
        let make_writer = MockMakeWriter::default();
        let subscriber = crate::fmt::Subscriber::builder()
            .with_writer(make_writer.clone())
            .with_level(false)
            .with_ansi(false)
            .with_timer(MockTime)
            .with_span_events(FmtSpan::ACTIVE)
            .finish();

        with_default(subscriber, || {
            let span1 = tracing::info_span!("span1", x = 42);
            let _e = span1.enter();
        });
        let actual = sanitize_timings(make_writer.get_string());
        assert_eq!(
            "fake time span1{x=42}: tracing_subscriber::fmt::fmt_layer::test: enter\n\
             fake time span1{x=42}: tracing_subscriber::fmt::fmt_layer::test: exit\n",
            actual.as_str()
        );
    }

    #[test]
    fn synthesize_span_close() {
        let make_writer = MockMakeWriter::default();
        let subscriber = crate::fmt::Subscriber::builder()
            .with_writer(make_writer.clone())
            .with_level(false)
            .with_ansi(false)
            .with_timer(MockTime)
            .with_span_events(FmtSpan::CLOSE)
            .finish();

        with_default(subscriber, || {
            let span1 = tracing::info_span!("span1", x = 42);
            let _e = span1.enter();
        });
        let actual = sanitize_timings(make_writer.get_string());
        assert_eq!(
            "fake time span1{x=42}: tracing_subscriber::fmt::fmt_layer::test: close timing timing\n",
            actual.as_str()
        );
    }

    #[test]
    fn synthesize_span_close_no_timing() {
        let make_writer = MockMakeWriter::default();
        let subscriber = crate::fmt::Subscriber::builder()
            .with_writer(make_writer.clone())
            .with_level(false)
            .with_ansi(false)
            .with_timer(MockTime)
            .without_time()
            .with_span_events(FmtSpan::CLOSE)
            .finish();

        with_default(subscriber, || {
            let span1 = tracing::info_span!("span1", x = 42);
            let _e = span1.enter();
        });
        let actual = sanitize_timings(make_writer.get_string());
        assert_eq!(
            "span1{x=42}: tracing_subscriber::fmt::fmt_layer::test: close\n",
            actual.as_str()
        );
    }

    #[test]
    fn synthesize_span_full() {
        let make_writer = MockMakeWriter::default();
        let subscriber = crate::fmt::Subscriber::builder()
            .with_writer(make_writer.clone())
            .with_level(false)
            .with_ansi(false)
            .with_timer(MockTime)
            .with_span_events(FmtSpan::FULL)
            .finish();

        with_default(subscriber, || {
            let span1 = tracing::info_span!("span1", x = 42);
            let _e = span1.enter();
        });
        let actual = sanitize_timings(make_writer.get_string());
        assert_eq!(
            "fake time span1{x=42}: tracing_subscriber::fmt::fmt_layer::test: new\n\
             fake time span1{x=42}: tracing_subscriber::fmt::fmt_layer::test: enter\n\
             fake time span1{x=42}: tracing_subscriber::fmt::fmt_layer::test: exit\n\
             fake time span1{x=42}: tracing_subscriber::fmt::fmt_layer::test: close timing timing\n",
            actual.as_str()
        );
    }

    #[test]
    fn make_writer_based_on_meta() {
        struct MakeByTarget {
            make_writer1: MockMakeWriter,
            make_writer2: MockMakeWriter,
        }

        impl<'a> MakeWriter<'a> for MakeByTarget {
            type Writer = MockWriter;

            fn make_writer(&'a self) -> Self::Writer {
                self.make_writer1.make_writer()
            }

            fn make_writer_for(&'a self, meta: &Metadata<'_>) -> Self::Writer {
                if meta.target() == "writer2" {
                    return self.make_writer2.make_writer();
                }
                self.make_writer()
            }
        }

        let make_writer1 = MockMakeWriter::default();
        let make_writer2 = MockMakeWriter::default();

        let make_writer = MakeByTarget {
            make_writer1: make_writer1.clone(),
            make_writer2: make_writer2.clone(),
        };

        let subscriber = crate::fmt::Subscriber::builder()
            .with_writer(make_writer)
            .with_level(false)
            .with_target(false)
            .with_ansi(false)
            .with_timer(MockTime)
            .with_span_events(FmtSpan::CLOSE)
            .finish();

        with_default(subscriber, || {
            let span1 = tracing::info_span!("writer1_span", x = 42);
            let _e = span1.enter();
            tracing::info!(target: "writer2", "hello writer2!");
            let span2 = tracing::info_span!(target: "writer2", "writer2_span");
            let _e = span2.enter();
            tracing::warn!(target: "writer1", "hello writer1!");
        });

        let actual = sanitize_timings(make_writer1.get_string());
        assert_eq!(
            "fake time writer1_span{x=42}:writer2_span: hello writer1!\n\
             fake time writer1_span{x=42}: close timing timing\n",
            actual.as_str()
        );
        let actual = sanitize_timings(make_writer2.get_string());
        assert_eq!(
            "fake time writer1_span{x=42}: hello writer2!\n\
             fake time writer1_span{x=42}:writer2_span: close timing timing\n",
            actual.as_str()
        );
    }

    // Because we need to modify an environment variable for these test cases,
    // we do them all in a single test.
    #[cfg(feature = "ansi")]
    #[test]
    fn layer_no_color() {
        const NO_COLOR: &str = "NO_COLOR";

        // Restores the previous value of the `NO_COLOR` env variable when
        // dropped.
        //
        // This is done in a `Drop` implementation, rather than just resetting
        // the value at the end of the test, so that the previous value is
        // restored even if the test panics.
        struct RestoreEnvVar(Result<String, env::VarError>);
        impl Drop for RestoreEnvVar {
            fn drop(&mut self) {
                match self.0 {
                    Ok(ref var) => env::set_var(NO_COLOR, var),
                    Err(_) => env::remove_var(NO_COLOR),
                }
            }
        }

        let _saved_no_color = RestoreEnvVar(env::var(NO_COLOR));

        let cases: Vec<(Option<&str>, bool)> = vec![
            (Some("0"), false),   // any non-empty value disables ansi
            (Some("off"), false), // any non-empty value disables ansi
            (Some("1"), false),
            (Some(""), true), // empty value does not disable ansi
            (None, true),
        ];

        for (var, ansi) in cases {
            if let Some(value) = var {
                env::set_var(NO_COLOR, value);
            } else {
                env::remove_var(NO_COLOR);
            }

            let layer: Layer<()> = fmt::Layer::default();
            assert_eq!(
                layer.is_ansi, ansi,
                "NO_COLOR={:?}; Layer::default().is_ansi should be {}",
                var, ansi
            );

            // with_ansi should override any `NO_COLOR` value
            let layer: Layer<()> = fmt::Layer::default().with_ansi(true);
            assert!(
                layer.is_ansi,
                "NO_COLOR={:?}; Layer::default().with_ansi(true).is_ansi should be true",
                var
            );

            // set_ansi should override any `NO_COLOR` value
            let mut layer: Layer<()> = fmt::Layer::default();
            layer.set_ansi(true);
            assert!(
                layer.is_ansi,
                "NO_COLOR={:?}; layer.set_ansi(true); layer.is_ansi should be true",
                var
            );
        }

        // dropping `_saved_no_color` will restore the previous value of
        // `NO_COLOR`.
    }

    // Validates that span event configuration can be modified with a reload handle
    #[test]
    fn modify_span_events() {
        let make_writer = MockMakeWriter::default();

        let inner_layer = fmt::Layer::default()
            .with_writer(make_writer.clone())
            .with_level(false)
            .with_ansi(false)
            .with_timer(MockTime)
            .with_span_events(FmtSpan::ACTIVE);

        let (reloadable_layer, reload_handle) = crate::reload::Layer::new(inner_layer);
        let reload = reloadable_layer.with_subscriber(Registry::default());

        with_default(reload, || {
            {
                let span1 = tracing::info_span!("span1", x = 42);
                let _e = span1.enter();
            }

            let _ = reload_handle.modify(|s| s.set_span_events(FmtSpan::NONE));

            // this span should not be logged at all!
            {
                let span2 = tracing::info_span!("span2", x = 100);
                let _e = span2.enter();
            }

            {
                let span3 = tracing::info_span!("span3", x = 42);
                let _e = span3.enter();

                // The span config was modified after span3 was already entered.
                // We should only see an exit
                let _ = reload_handle.modify(|s| s.set_span_events(FmtSpan::ACTIVE));
            }
        });
        let actual = sanitize_timings(make_writer.get_string());
        assert_eq!(
            "fake time span1{x=42}: tracing_subscriber::fmt::fmt_layer::test: enter\n\
             fake time span1{x=42}: tracing_subscriber::fmt::fmt_layer::test: exit\n\
             fake time span3{x=42}: tracing_subscriber::fmt::fmt_layer::test: exit\n",
            actual.as_str()
        );
    }
}
