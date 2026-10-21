#[cfg(test)]
mod test {
    use super::*;
    use crate::fmt::format::Format;
    use crate::fmt::test::{MockMakeWriter, MockWriter};
    use crate::fmt::Subscriber;
    use alloc::vec::Vec;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::{dbg, format, println};
    use tracing::{debug, error, info, trace, warn, Level};
    use tracing_core::dispatcher::{self, Dispatch};

    fn test_writer<T>(make_writer: T, msg: &str, buf: &Mutex<Vec<u8>>)
    where
        T: for<'writer> MakeWriter<'writer> + Send + Sync + 'static,
    {
        let subscriber = {
            #[cfg(feature = "ansi")]
            let f = Format::default().without_time().with_ansi(false);
            #[cfg(not(feature = "ansi"))]
            let f = Format::default().without_time();
            Subscriber::builder()
                .event_format(f)
                .with_writer(make_writer)
                .finish()
        };
        let dispatch = Dispatch::from(subscriber);

        dispatcher::with_default(&dispatch, || {
            error!("{}", msg);
        });

        let expected = format!("ERROR {}: {}\n", module_path!(), msg);
        let actual = String::from_utf8(buf.try_lock().unwrap().to_vec()).unwrap();
        assert!(actual.contains(expected.as_str()));
    }

    fn has_lines(buf: &Mutex<Vec<u8>>, msgs: &[(tracing::Level, &str)]) {
        let actual = String::from_utf8(buf.try_lock().unwrap().to_vec()).unwrap();
        let mut expected_lines = msgs.iter();
        for line in actual.lines() {
            let line = dbg!(line).trim();
            let (level, msg) = expected_lines
                .next()
                .unwrap_or_else(|| panic!("expected no more lines, but got: {:?}", line));
            let expected = format!("{} {}: {}", level, module_path!(), msg);
            assert_eq!(line, expected.as_str());
        }
    }

    #[test]
    fn custom_writer_closure() {
        let buf = Arc::new(Mutex::new(Vec::new()));
        let buf2 = buf.clone();
        let make_writer = move || MockWriter::new(buf2.clone());
        let msg = "my custom writer closure error";
        test_writer(make_writer, msg, &buf);
    }

    #[test]
    fn custom_writer_struct() {
        let buf = Arc::new(Mutex::new(Vec::new()));
        let make_writer = MockMakeWriter::new(buf.clone());
        let msg = "my custom writer struct error";
        test_writer(make_writer, msg, &buf);
    }

    #[test]
    fn custom_writer_mutex() {
        let buf = Arc::new(Mutex::new(Vec::new()));
        let writer = MockWriter::new(buf.clone());
        let make_writer = Mutex::new(writer);
        let msg = "my mutex writer error";
        test_writer(make_writer, msg, &buf);
    }

    #[test]
    fn combinators_level_filters() {
        let info_buf = Arc::new(Mutex::new(Vec::new()));
        let info = MockMakeWriter::new(info_buf.clone());

        let debug_buf = Arc::new(Mutex::new(Vec::new()));
        let debug = MockMakeWriter::new(debug_buf.clone());

        let warn_buf = Arc::new(Mutex::new(Vec::new()));
        let warn = MockMakeWriter::new(warn_buf.clone());

        let err_buf = Arc::new(Mutex::new(Vec::new()));
        let err = MockMakeWriter::new(err_buf.clone());

        let make_writer = info
            .with_max_level(Level::INFO)
            .and(debug.with_max_level(Level::DEBUG))
            .and(warn.with_max_level(Level::WARN))
            .and(err.with_max_level(Level::ERROR));

        let c = {
            #[cfg(feature = "ansi")]
            let f = Format::default().without_time().with_ansi(false);
            #[cfg(not(feature = "ansi"))]
            let f = Format::default().without_time();
            Subscriber::builder()
                .event_format(f)
                .with_writer(make_writer)
                .with_max_level(Level::TRACE)
                .finish()
        };

        let _s = tracing::subscriber::set_default(c);

        trace!("trace");
        debug!("debug");
        info!("info");
        warn!("warn");
        error!("error");

        let all_lines = [
            (Level::TRACE, "trace"),
            (Level::DEBUG, "debug"),
            (Level::INFO, "info"),
            (Level::WARN, "warn"),
            (Level::ERROR, "error"),
        ];

        println!("max level debug");
        has_lines(&debug_buf, &all_lines[1..]);

        println!("max level info");
        has_lines(&info_buf, &all_lines[2..]);

        println!("max level warn");
        has_lines(&warn_buf, &all_lines[3..]);

        println!("max level error");
        has_lines(&err_buf, &all_lines[4..]);
    }

    #[test]
    fn combinators_or_else() {
        let some_buf = Arc::new(Mutex::new(Vec::new()));
        let some = MockMakeWriter::new(some_buf.clone());

        let or_else_buf = Arc::new(Mutex::new(Vec::new()));
        let or_else = MockMakeWriter::new(or_else_buf.clone());

        let return_some = AtomicBool::new(true);
        let make_writer = move || {
            if return_some.swap(false, Ordering::Relaxed) {
                OptionalWriter::some(some.make_writer())
            } else {
                OptionalWriter::none()
            }
        };
        let make_writer = make_writer.or_else(or_else);
        let c = {
            #[cfg(feature = "ansi")]
            let f = Format::default().without_time().with_ansi(false);
            #[cfg(not(feature = "ansi"))]
            let f = Format::default().without_time();
            Subscriber::builder()
                .event_format(f)
                .with_writer(make_writer)
                .with_max_level(Level::TRACE)
                .finish()
        };

        let _s = tracing::subscriber::set_default(c);
        info!("hello");
        info!("world");
        info!("goodbye");

        has_lines(&some_buf, &[(Level::INFO, "hello")]);
        has_lines(
            &or_else_buf,
            &[(Level::INFO, "world"), (Level::INFO, "goodbye")],
        );
    }

    #[test]
    fn combinators_or_else_chain() {
        let info_buf = Arc::new(Mutex::new(Vec::new()));
        let info = MockMakeWriter::new(info_buf.clone());

        let debug_buf = Arc::new(Mutex::new(Vec::new()));
        let debug = MockMakeWriter::new(debug_buf.clone());

        let warn_buf = Arc::new(Mutex::new(Vec::new()));
        let warn = MockMakeWriter::new(warn_buf.clone());

        let err_buf = Arc::new(Mutex::new(Vec::new()));
        let err = MockMakeWriter::new(err_buf.clone());

        let make_writer = err.with_max_level(Level::ERROR).or_else(
            warn.with_max_level(Level::WARN).or_else(
                info.with_max_level(Level::INFO)
                    .or_else(debug.with_max_level(Level::DEBUG)),
            ),
        );

        let c = {
            #[cfg(feature = "ansi")]
            let f = Format::default().without_time().with_ansi(false);
            #[cfg(not(feature = "ansi"))]
            let f = Format::default().without_time();
            Subscriber::builder()
                .event_format(f)
                .with_writer(make_writer)
                .with_max_level(Level::TRACE)
                .finish()
        };

        let _s = tracing::subscriber::set_default(c);

        trace!("trace");
        debug!("debug");
        info!("info");
        warn!("warn");
        error!("error");

        println!("max level debug");
        has_lines(&debug_buf, &[(Level::DEBUG, "debug")]);

        println!("max level info");
        has_lines(&info_buf, &[(Level::INFO, "info")]);

        println!("max level warn");
        has_lines(&warn_buf, &[(Level::WARN, "warn")]);

        println!("max level error");
        has_lines(&err_buf, &[(Level::ERROR, "error")]);
    }

    #[test]
    fn combinators_and() {
        let a_buf = Arc::new(Mutex::new(Vec::new()));
        let a = MockMakeWriter::new(a_buf.clone());

        let b_buf = Arc::new(Mutex::new(Vec::new()));
        let b = MockMakeWriter::new(b_buf.clone());

        let lines = &[(Level::INFO, "hello"), (Level::INFO, "world")];

        let make_writer = a.and(b);
        let c = {
            #[cfg(feature = "ansi")]
            let f = Format::default().without_time().with_ansi(false);
            #[cfg(not(feature = "ansi"))]
            let f = Format::default().without_time();
            Subscriber::builder()
                .event_format(f)
                .with_writer(make_writer)
                .with_max_level(Level::TRACE)
                .finish()
        };

        let _s = tracing::subscriber::set_default(c);
        info!("hello");
        info!("world");

        has_lines(&a_buf, &lines[..]);
        has_lines(&b_buf, &lines[..]);
    }
}
