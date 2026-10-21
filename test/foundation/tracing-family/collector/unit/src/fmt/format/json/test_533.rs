#[cfg(test)]
mod test {
    use super::*;
    use crate::collector::fmt::{format::FmtSpan, test::MockMakeWriter, time::FormatTime, SubscriberBuilder};
    use tracing::{self, subscriber::with_default};

    use std::fmt;
    use std::path::Path;

    struct MockTime;
    impl FormatTime for MockTime {
        fn format_time(&self, w: &mut Writer<'_>) -> fmt::Result {
            write!(w, "fake time")
        }
    }

    fn subscriber() -> SubscriberBuilder<JsonFields, Format<Json>> {
        SubscriberBuilder::default().json()
    }

    #[test]
    fn json() {
        let expected =
        "{\"timestamp\":\"fake time\",\"level\":\"INFO\",\"span\":{\"answer\":42,\"name\":\"json_span\",\"number\":3,\"slice\":[97,98,99]},\"spans\":[{\"answer\":42,\"name\":\"json_span\",\"number\":3,\"slice\":[97,98,99]}],\"target\":\"tracing::collector::fmt::format::json::test\",\"fields\":{\"message\":\"some json test\"}}\n";
        let subscriber = subscriber()
            .flatten_event(false)
            .with_current_span(true)
            .with_span_list(true);
        test_json(expected, subscriber, || {
            let span = tracing::span!(
                tracing::Level::INFO,
                "json_span",
                answer = 42,
                number = 3,
                slice = &b"abc"[..]
            );
            let _guard = span.enter();
            tracing::info!("some json test");
        });
    }

    #[test]
    fn json_filename() {
        let current_path = Path::new(file!())
            .to_str()
            .expect("path must be valid unicode")
            // escape windows backslashes
            .replace('\\', "\\\\");
        let expected =
            &format!("{}{}{}",
                    "{\"timestamp\":\"fake time\",\"level\":\"INFO\",\"span\":{\"answer\":42,\"name\":\"json_span\",\"number\":3},\"spans\":[{\"answer\":42,\"name\":\"json_span\",\"number\":3}],\"target\":\"tracing::collector::fmt::format::json::test\",\"filename\":\"",
                    current_path,
                    "\",\"fields\":{\"message\":\"some json test\"}}\n");
        let subscriber = subscriber()
            .flatten_event(false)
            .with_current_span(true)
            .with_file(true)
            .with_span_list(true);
        test_json(expected, subscriber, || {
            let span = tracing::span!(tracing::Level::INFO, "json_span", answer = 42, number = 3);
            let _guard = span.enter();
            tracing::info!("some json test");
        });
    }

    #[test]
    fn json_line_number() {
        let expected =
            "{\"timestamp\":\"fake time\",\"level\":\"INFO\",\"span\":{\"answer\":42,\"name\":\"json_span\",\"number\":3},\"spans\":[{\"answer\":42,\"name\":\"json_span\",\"number\":3}],\"target\":\"tracing::collector::fmt::format::json::test\",\"line_number\":42,\"fields\":{\"message\":\"some json test\"}}\n";
        let subscriber = subscriber()
            .flatten_event(false)
            .with_current_span(true)
            .with_line_number(true)
            .with_span_list(true);
        test_json_with_line_number(expected, subscriber, || {
            let span = tracing::span!(tracing::Level::INFO, "json_span", answer = 42, number = 3);
            let _guard = span.enter();
            tracing::info!("some json test");
        });
    }

    #[test]
    fn json_flattened_event() {
        let expected =
        "{\"timestamp\":\"fake time\",\"level\":\"INFO\",\"span\":{\"answer\":42,\"name\":\"json_span\",\"number\":3},\"spans\":[{\"answer\":42,\"name\":\"json_span\",\"number\":3}],\"target\":\"tracing::collector::fmt::format::json::test\",\"message\":\"some json test\"}\n";

        let subscriber = subscriber()
            .flatten_event(true)
            .with_current_span(true)
            .with_span_list(true);
        test_json(expected, subscriber, || {
            let span = tracing::span!(tracing::Level::INFO, "json_span", answer = 42, number = 3);
            let _guard = span.enter();
            tracing::info!("some json test");
        });
    }

    #[test]
    fn json_disabled_current_span_event() {
        let expected =
        "{\"timestamp\":\"fake time\",\"level\":\"INFO\",\"spans\":[{\"answer\":42,\"name\":\"json_span\",\"number\":3}],\"target\":\"tracing::collector::fmt::format::json::test\",\"fields\":{\"message\":\"some json test\"}}\n";
        let subscriber = subscriber()
            .flatten_event(false)
            .with_current_span(false)
            .with_span_list(true);
        test_json(expected, subscriber, || {
            let span = tracing::span!(tracing::Level::INFO, "json_span", answer = 42, number = 3);
            let _guard = span.enter();
            tracing::info!("some json test");
        });
    }

    #[test]
    fn json_disabled_span_list_event() {
        let expected =
        "{\"timestamp\":\"fake time\",\"level\":\"INFO\",\"span\":{\"answer\":42,\"name\":\"json_span\",\"number\":3},\"target\":\"tracing::collector::fmt::format::json::test\",\"fields\":{\"message\":\"some json test\"}}\n";
        let subscriber = subscriber()
            .flatten_event(false)
            .with_current_span(true)
            .with_span_list(false);
        test_json(expected, subscriber, || {
            let span = tracing::span!(tracing::Level::INFO, "json_span", answer = 42, number = 3);
            let _guard = span.enter();
            tracing::info!("some json test");
        });
    }

    #[test]
    fn json_nested_span() {
        let expected =
        "{\"timestamp\":\"fake time\",\"level\":\"INFO\",\"span\":{\"answer\":43,\"name\":\"nested_json_span\",\"number\":4},\"spans\":[{\"answer\":42,\"name\":\"json_span\",\"number\":3},{\"answer\":43,\"name\":\"nested_json_span\",\"number\":4}],\"target\":\"tracing::collector::fmt::format::json::test\",\"fields\":{\"message\":\"some json test\"}}\n";
        let subscriber = subscriber()
            .flatten_event(false)
            .with_current_span(true)
            .with_span_list(true);
        test_json(expected, subscriber, || {
            let span = tracing::span!(tracing::Level::INFO, "json_span", answer = 42, number = 3);
            let _guard = span.enter();
            let span = tracing::span!(
                tracing::Level::INFO,
                "nested_json_span",
                answer = 43,
                number = 4
            );
            let _guard = span.enter();
            tracing::info!("some json test");
        });
    }

    #[test]
    fn json_no_span() {
        let expected =
        "{\"timestamp\":\"fake time\",\"level\":\"INFO\",\"target\":\"tracing::collector::fmt::format::json::test\",\"fields\":{\"message\":\"some json test\"}}\n";
        let subscriber = subscriber()
            .flatten_event(false)
            .with_current_span(true)
            .with_span_list(true);
        test_json(expected, subscriber, || {
            tracing::info!("some json test");
        });
    }

    #[test]
    fn record_works() {
        // This test reproduces issue #707, where using `Span::record` causes
        // any events inside the span to be ignored.

        let make_writer = MockMakeWriter::default();
        let subscriber = crate::collector::fmt()
            .json()
            .with_writer(make_writer.clone())
            .finish();

        with_default(subscriber, || {
            tracing::info!("an event outside the root span");
            assert_eq!(
                parse_as_json(&make_writer)["fields"]["message"],
                "an event outside the root span"
            );

            let span = tracing::info_span!("the span", na = tracing::field::Empty);
            span.record("na", "value");
            let _enter = span.enter();

            tracing::info!("an event inside the root span");
            assert_eq!(
                parse_as_json(&make_writer)["fields"]["message"],
                "an event inside the root span"
            );
        });
    }

    #[test]
    fn json_span_event_show_correct_context() {
        let buffer = MockMakeWriter::default();
        let subscriber = subscriber()
            .with_writer(buffer.clone())
            .flatten_event(false)
            .with_current_span(true)
            .with_span_list(false)
            .with_span_events(FmtSpan::FULL)
            .finish();

        with_default(subscriber, || {
            let context = "parent";
            let parent_span = tracing::info_span!("parent_span", context);

            let event = parse_as_json(&buffer);
            assert_eq!(event["fields"]["message"], "new");
            assert_eq!(event["span"]["context"], "parent");

            let _parent_enter = parent_span.enter();
            let event = parse_as_json(&buffer);
            assert_eq!(event["fields"]["message"], "enter");
            assert_eq!(event["span"]["context"], "parent");

            let context = "child";
            let child_span = tracing::info_span!("child_span", context);
            let event = parse_as_json(&buffer);
            assert_eq!(event["fields"]["message"], "new");
            assert_eq!(event["span"]["context"], "child");

            let _child_enter = child_span.enter();
            let event = parse_as_json(&buffer);
            assert_eq!(event["fields"]["message"], "enter");
            assert_eq!(event["span"]["context"], "child");

            drop(_child_enter);
            let event = parse_as_json(&buffer);
            assert_eq!(event["fields"]["message"], "exit");
            assert_eq!(event["span"]["context"], "child");

            drop(child_span);
            let event = parse_as_json(&buffer);
            assert_eq!(event["fields"]["message"], "close");
            assert_eq!(event["span"]["context"], "child");

            drop(_parent_enter);
            let event = parse_as_json(&buffer);
            assert_eq!(event["fields"]["message"], "exit");
            assert_eq!(event["span"]["context"], "parent");

            drop(parent_span);
            let event = parse_as_json(&buffer);
            assert_eq!(event["fields"]["message"], "close");
            assert_eq!(event["span"]["context"], "parent");
        });
    }

    #[test]
    fn json_span_event_with_no_fields() {
        // Check span events serialize correctly.
        // Discussion: https://github.com/tokio-rs/tracing/issues/829#issuecomment-661984255
        let buffer = MockMakeWriter::default();
        let subscriber = subscriber()
            .with_writer(buffer.clone())
            .flatten_event(false)
            .with_current_span(false)
            .with_span_list(false)
            .with_span_events(FmtSpan::FULL)
            .finish();

        with_default(subscriber, || {
            let span = tracing::info_span!("valid_json");
            assert_eq!(parse_as_json(&buffer)["fields"]["message"], "new");

            let _enter = span.enter();
            assert_eq!(parse_as_json(&buffer)["fields"]["message"], "enter");

            drop(_enter);
            assert_eq!(parse_as_json(&buffer)["fields"]["message"], "exit");

            drop(span);
            assert_eq!(parse_as_json(&buffer)["fields"]["message"], "close");
        });
    }

    fn parse_as_json(buffer: &MockMakeWriter) -> json_codec::Value {
        let buf = String::from_utf8(buffer.buf().to_vec()).unwrap();
        let json = buf
            .lines()
            .last()
            .expect("expected at least one line to be written!");
        match json_codec::from_str(json) {
            Ok(v) => v,
            Err(e) => panic!(
                "assertion failed: JSON shouldn't be malformed\n  error: {}\n  json: {}",
                e, json
            ),
        }
    }

    fn test_json<T>(
        expected: &str,
        builder: crate::collector::fmt::SubscriberBuilder<JsonFields, Format<Json>>,
        producer: impl FnOnce() -> T,
    ) {
        let make_writer = MockMakeWriter::default();
        let subscriber = builder
            .with_writer(make_writer.clone())
            .with_timer(MockTime)
            .finish();

        with_default(subscriber, producer);

        let buf = make_writer.buf();
        let actual = std::str::from_utf8(&buf[..]).unwrap();
        assert_eq!(
            json_codec::from_str::<std::collections::HashMap<&str, json_codec::Value>>(expected)
                .unwrap(),
            json_codec::from_str(actual).unwrap()
        );
    }

    fn test_json_with_line_number<T>(
        expected: &str,
        builder: crate::collector::fmt::SubscriberBuilder<JsonFields, Format<Json>>,
        producer: impl FnOnce() -> T,
    ) {
        let make_writer = MockMakeWriter::default();
        let subscriber = builder
            .with_writer(make_writer.clone())
            .with_timer(MockTime)
            .finish();

        with_default(subscriber, producer);

        let buf = make_writer.buf();
        let actual = std::str::from_utf8(&buf[..]).unwrap();
        let mut expected =
            json_codec::from_str::<std::collections::HashMap<&str, json_codec::Value>>(expected)
                .unwrap();
        let expect_line_number = expected.remove("line_number").is_some();
        let mut actual: std::collections::HashMap<&str, json_codec::Value> =
            json_codec::from_str(actual).unwrap();
        let line_number = actual.remove("line_number");
        if expect_line_number {
            assert_eq!(line_number.map(|x| x.is_number()), Some(true));
        } else {
            assert!(line_number.is_none());
        }
        assert_eq!(actual, expected);
    }
}
