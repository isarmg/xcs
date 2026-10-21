#[cfg(test)]
pub(super) mod test {
    use crate::fmt::{test::MockMakeWriter, time::FormatTime};
    use alloc::{
        borrow::ToOwned,
        format,
        string::{String, ToString},
    };
    use tracing::{
        self,
        dispatcher::{set_default, Dispatch},
        subscriber::with_default,
    };

    use super::*;

    use regex::Regex;
    use std::{fmt, path::Path};

    pub(crate) struct MockTime;
    impl FormatTime for MockTime {
        fn format_time(&self, w: &mut Writer<'_>) -> fmt::Result {
            write!(w, "fake time")
        }
    }

    #[test]
    fn disable_everything() {
        // This test reproduces https://github.com/tokio-rs/tracing/issues/1354
        let make_writer = MockMakeWriter::default();
        let subscriber = crate::fmt::Subscriber::builder()
            .with_writer(make_writer.clone())
            .without_time()
            .with_level(false)
            .with_target(false)
            .with_thread_ids(false)
            .with_thread_names(false);
        #[cfg(feature = "ansi")]
        let subscriber = subscriber.with_ansi(false);
        assert_info_hello(subscriber, make_writer, "hello\n")
    }

    fn test_ansi<T>(
        is_ansi: bool,
        expected: &str,
        builder: crate::fmt::SubscriberBuilder<DefaultFields, Format<T>>,
    ) where
        Format<T, MockTime>: FormatEvent<crate::Registry, DefaultFields>,
        T: Send + Sync + 'static,
    {
        let make_writer = MockMakeWriter::default();
        let subscriber = builder
            .with_writer(make_writer.clone())
            .with_ansi(is_ansi)
            .with_timer(MockTime);
        run_test(subscriber, make_writer, expected)
    }

    #[cfg(not(feature = "ansi"))]
    fn test_without_ansi<T>(
        expected: &str,
        builder: crate::fmt::SubscriberBuilder<DefaultFields, Format<T>>,
    ) where
        Format<T, MockTime>: FormatEvent<crate::Registry, DefaultFields>,
        T: Send + Sync,
    {
        let make_writer = MockMakeWriter::default();
        let subscriber = builder.with_writer(make_writer).with_timer(MockTime);
        run_test(subscriber, make_writer, expected)
    }

    fn test_without_level<T>(
        expected: &str,
        builder: crate::fmt::SubscriberBuilder<DefaultFields, Format<T>>,
    ) where
        Format<T, MockTime>: FormatEvent<crate::Registry, DefaultFields>,
        T: Send + Sync + 'static,
    {
        let make_writer = MockMakeWriter::default();
        let subscriber = builder
            .with_writer(make_writer.clone())
            .with_level(false)
            .with_ansi(false)
            .with_timer(MockTime);
        run_test(subscriber, make_writer, expected);
    }

    #[test]
    fn with_line_number_and_file_name() {
        let make_writer = MockMakeWriter::default();
        let subscriber = crate::fmt::Subscriber::builder()
            .with_writer(make_writer.clone())
            .with_file(true)
            .with_line_number(true)
            .with_level(false)
            .with_ansi(false)
            .with_timer(MockTime);

        let expected = Regex::new(&format!(
            "^fake time tracing_subscriber::fmt::format::test: {}:[0-9]+: hello\n$",
            current_path()
                // if we're on Windows, the path might contain backslashes, which
                // have to be escaped before compiling the regex.
                .replace('\\', "\\\\")
        ))
        .unwrap();
        let _default = set_default(&subscriber.into());
        tracing::info!("hello");
        let res = make_writer.get_string();
        assert!(expected.is_match(&res));
    }

    #[test]
    fn with_line_number() {
        let make_writer = MockMakeWriter::default();
        let subscriber = crate::fmt::Subscriber::builder()
            .with_writer(make_writer.clone())
            .with_line_number(true)
            .with_level(false)
            .with_ansi(false)
            .with_timer(MockTime);

        let expected =
            Regex::new("^fake time tracing_subscriber::fmt::format::test: [0-9]+: hello\n$")
                .unwrap();
        let _default = set_default(&subscriber.into());
        tracing::info!("hello");
        let res = make_writer.get_string();
        assert!(expected.is_match(&res));
    }

    #[test]
    fn with_filename() {
        let make_writer = MockMakeWriter::default();
        let subscriber = crate::fmt::Subscriber::builder()
            .with_writer(make_writer.clone())
            .with_file(true)
            .with_level(false)
            .with_ansi(false)
            .with_timer(MockTime);
        let expected = &format!(
            "fake time tracing_subscriber::fmt::format::test: {}: hello\n",
            current_path(),
        );
        assert_info_hello(subscriber, make_writer, expected);
    }

    #[test]
    fn with_thread_ids() {
        let make_writer = MockMakeWriter::default();
        let subscriber = crate::fmt::Subscriber::builder()
            .with_writer(make_writer.clone())
            .with_thread_ids(true)
            .with_ansi(false)
            .with_timer(MockTime);
        let expected =
            "fake time  INFO ThreadId(NUMERIC) tracing_subscriber::fmt::format::test: hello\n";

        assert_info_hello_ignore_numeric(subscriber, make_writer, expected);
    }

    #[test]
    fn pretty_default() {
        let make_writer = MockMakeWriter::default();
        let subscriber = crate::fmt::Subscriber::builder()
            .pretty()
            .with_writer(make_writer.clone())
            .with_ansi(false)
            .with_timer(MockTime);
        let expected = format!(
            r#"  fake time  INFO tracing_subscriber::fmt::format::test: hello
    at {}:NUMERIC

"#,
            file!()
        );

        assert_info_hello_ignore_numeric(subscriber, make_writer, &expected)
    }

    fn assert_info_hello(subscriber: impl Into<Dispatch>, buf: MockMakeWriter, expected: &str) {
        let _default = set_default(&subscriber.into());
        tracing::info!("hello");
        let result = buf.get_string();

        assert_eq!(expected, result)
    }

    // When numeric characters are used they often form a non-deterministic value as they usually represent things like a thread id or line number.
    // This assert method should be used when non-deterministic numeric characters are present.
    fn assert_info_hello_ignore_numeric(
        subscriber: impl Into<Dispatch>,
        buf: MockMakeWriter,
        expected: &str,
    ) {
        let _default = set_default(&subscriber.into());
        tracing::info!("hello");

        let regex = Regex::new("[0-9]+").unwrap();
        let result = buf.get_string();
        let result_cleaned = regex.replace_all(&result, "NUMERIC");

        assert_eq!(expected, result_cleaned)
    }

    fn test_overridden_parents<T>(
        expected: &str,
        builder: crate::fmt::SubscriberBuilder<DefaultFields, Format<T>>,
    ) where
        Format<T, MockTime>: FormatEvent<crate::Registry, DefaultFields>,
        T: Send + Sync + 'static,
    {
        let make_writer = MockMakeWriter::default();
        let subscriber = builder
            .with_writer(make_writer.clone())
            .with_level(false)
            .with_ansi(false)
            .with_timer(MockTime)
            .finish();

        with_default(subscriber, || {
            let span1 = tracing::info_span!("span1", span = 1);
            let span2 = tracing::info_span!(parent: &span1, "span2", span = 2);
            tracing::info!(parent: &span2, "hello");
        });
        assert_eq!(expected, make_writer.get_string());
    }

    fn test_overridden_parents_in_scope<T>(
        expected1: &str,
        expected2: &str,
        builder: crate::fmt::SubscriberBuilder<DefaultFields, Format<T>>,
    ) where
        Format<T, MockTime>: FormatEvent<crate::Registry, DefaultFields>,
        T: Send + Sync + 'static,
    {
        let make_writer = MockMakeWriter::default();
        let subscriber = builder
            .with_writer(make_writer.clone())
            .with_level(false)
            .with_ansi(false)
            .with_timer(MockTime)
            .finish();

        with_default(subscriber, || {
            let span1 = tracing::info_span!("span1", span = 1);
            let span2 = tracing::info_span!(parent: &span1, "span2", span = 2);
            let span3 = tracing::info_span!("span3", span = 3);
            let _e3 = span3.enter();

            tracing::info!("hello");
            assert_eq!(expected1, make_writer.get_string().as_str());

            tracing::info!(parent: &span2, "hello");
            assert_eq!(expected2, make_writer.get_string().as_str());
        });
    }

    fn run_test(subscriber: impl Into<Dispatch>, buf: MockMakeWriter, expected: &str) {
        let _default = set_default(&subscriber.into());
        tracing::info!("hello");
        assert_eq!(expected, buf.get_string())
    }

    mod default {
        use super::*;

        #[test]
        fn with_thread_ids() {
            let make_writer = MockMakeWriter::default();
            let subscriber = crate::fmt::Subscriber::builder()
                .with_writer(make_writer.clone())
                .with_thread_ids(true)
                .with_ansi(false)
                .with_timer(MockTime);
            let expected =
                "fake time  INFO ThreadId(NUMERIC) tracing_subscriber::fmt::format::test: hello\n";

            assert_info_hello_ignore_numeric(subscriber, make_writer, expected);
        }

        #[cfg(feature = "ansi")]
        #[test]
        fn with_ansi_true() {
            let expected = "\u{1b}[2mfake time\u{1b}[0m \u{1b}[32m INFO\u{1b}[0m \u{1b}[2mtracing_subscriber::fmt::format::test\u{1b}[0m\u{1b}[2m:\u{1b}[0m hello\n";
            test_ansi(true, expected, crate::fmt::Subscriber::builder());
        }

        #[cfg(feature = "ansi")]
        #[test]
        fn with_ansi_false() {
            let expected = "fake time  INFO tracing_subscriber::fmt::format::test: hello\n";
            test_ansi(false, expected, crate::fmt::Subscriber::builder());
        }

        #[cfg(not(feature = "ansi"))]
        #[test]
        fn without_ansi() {
            let expected = "fake time  INFO tracing_subscriber::fmt::format::test: hello\n";
            test_without_ansi(expected, crate::fmt::Subscriber::builder())
        }

        #[test]
        fn without_level() {
            let expected = "fake time tracing_subscriber::fmt::format::test: hello\n";
            test_without_level(expected, crate::fmt::Subscriber::builder())
        }

        #[test]
        fn overridden_parents() {
            let expected = "fake time span1{span=1}:span2{span=2}: tracing_subscriber::fmt::format::test: hello\n";
            test_overridden_parents(expected, crate::fmt::Subscriber::builder())
        }

        #[test]
        fn overridden_parents_in_scope() {
            test_overridden_parents_in_scope(
                "fake time span3{span=3}: tracing_subscriber::fmt::format::test: hello\n",
                "fake time span1{span=1}:span2{span=2}: tracing_subscriber::fmt::format::test: hello\n",
                crate::fmt::Subscriber::builder(),
            )
        }
    }

    mod compact {
        use super::*;

        #[cfg(feature = "ansi")]
        #[test]
        fn with_ansi_true() {
            let expected = "\u{1b}[2mfake time\u{1b}[0m \u{1b}[32m INFO\u{1b}[0m \u{1b}[2mtracing_subscriber::fmt::format::test\u{1b}[0m\u{1b}[2m:\u{1b}[0m hello\n";
            test_ansi(true, expected, crate::fmt::Subscriber::builder().compact())
        }

        #[cfg(feature = "ansi")]
        #[test]
        fn with_ansi_false() {
            let expected = "fake time  INFO tracing_subscriber::fmt::format::test: hello\n";
            test_ansi(false, expected, crate::fmt::Subscriber::builder().compact());
        }

        #[cfg(not(feature = "ansi"))]
        #[test]
        fn without_ansi() {
            let expected = "fake time  INFO tracing_subscriber::fmt::format::test: hello\n";
            test_without_ansi(expected, crate::fmt::Subscriber::builder().compact())
        }

        #[test]
        fn without_level() {
            let expected = "fake time tracing_subscriber::fmt::format::test: hello\n";
            test_without_level(expected, crate::fmt::Subscriber::builder().compact());
        }

        #[test]
        fn overridden_parents() {
            let expected = "fake time span1:span2: tracing_subscriber::fmt::format::test: hello span=1 span=2\n";
            test_overridden_parents(expected, crate::fmt::Subscriber::builder().compact())
        }

        #[test]
        fn overridden_parents_in_scope() {
            test_overridden_parents_in_scope(
                "fake time span3: tracing_subscriber::fmt::format::test: hello span=3\n",
                "fake time span1:span2: tracing_subscriber::fmt::format::test: hello span=1 span=2\n",
                crate::fmt::Subscriber::builder().compact(),
            )
        }
    }

    mod pretty {
        use super::*;

        #[test]
        fn pretty_default() {
            let make_writer = MockMakeWriter::default();
            let subscriber = crate::fmt::Subscriber::builder()
                .pretty()
                .with_writer(make_writer.clone())
                .with_ansi(false)
                .with_timer(MockTime);
            let expected = format!(
                "  fake time  INFO tracing_subscriber::fmt::format::test: hello\n    at {}:NUMERIC\n\n",
                file!()
            );

            assert_info_hello_ignore_numeric(subscriber, make_writer, &expected)
        }
    }

    #[test]
    fn format_nanos() {
        fn fmt(t: u64) -> String {
            TimingDisplay(t).to_string()
        }

        assert_eq!(fmt(1), "1.00ns");
        assert_eq!(fmt(12), "12.0ns");
        assert_eq!(fmt(123), "123ns");
        assert_eq!(fmt(1234), "1.23µs");
        assert_eq!(fmt(12345), "12.3µs");
        assert_eq!(fmt(123456), "123µs");
        assert_eq!(fmt(1234567), "1.23ms");
        assert_eq!(fmt(12345678), "12.3ms");
        assert_eq!(fmt(123456789), "123ms");
        assert_eq!(fmt(1234567890), "1.23s");
        assert_eq!(fmt(12345678901), "12.3s");
        assert_eq!(fmt(123456789012), "123s");
        assert_eq!(fmt(1234567890123), "1235s");
    }

    #[test]
    fn fmt_span_combinations() {
        let f = FmtSpan::NONE;
        assert!(!f.contains(FmtSpan::NEW));
        assert!(!f.contains(FmtSpan::ENTER));
        assert!(!f.contains(FmtSpan::EXIT));
        assert!(!f.contains(FmtSpan::CLOSE));

        let f = FmtSpan::ACTIVE;
        assert!(!f.contains(FmtSpan::NEW));
        assert!(f.contains(FmtSpan::ENTER));
        assert!(f.contains(FmtSpan::EXIT));
        assert!(!f.contains(FmtSpan::CLOSE));

        let f = FmtSpan::FULL;
        assert!(f.contains(FmtSpan::NEW));
        assert!(f.contains(FmtSpan::ENTER));
        assert!(f.contains(FmtSpan::EXIT));
        assert!(f.contains(FmtSpan::CLOSE));

        let f = FmtSpan::NEW | FmtSpan::CLOSE;
        assert!(f.contains(FmtSpan::NEW));
        assert!(!f.contains(FmtSpan::ENTER));
        assert!(!f.contains(FmtSpan::EXIT));
        assert!(f.contains(FmtSpan::CLOSE));
    }

    /// Returns the test's module path.
    fn current_path() -> String {
        Path::new("tracing-subscriber")
            .join("src")
            .join("fmt")
            .join("format")
            .join("mod.rs")
            .to_str()
            .expect("path must not contain invalid unicode")
            .to_owned()
    }
}
