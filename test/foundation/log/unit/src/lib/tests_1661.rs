#[cfg(test)]
mod tests {
    use super::{Level, LevelFilter, ParseLevelError, STATIC_MAX_LEVEL};

    #[test]
    fn test_levelfilter_from_str() {
        let tests = [
            ("off", Ok(LevelFilter::Off)),
            ("error", Ok(LevelFilter::Error)),
            ("warn", Ok(LevelFilter::Warn)),
            ("info", Ok(LevelFilter::Info)),
            ("debug", Ok(LevelFilter::Debug)),
            ("trace", Ok(LevelFilter::Trace)),
            ("OFF", Ok(LevelFilter::Off)),
            ("ERROR", Ok(LevelFilter::Error)),
            ("WARN", Ok(LevelFilter::Warn)),
            ("INFO", Ok(LevelFilter::Info)),
            ("DEBUG", Ok(LevelFilter::Debug)),
            ("TRACE", Ok(LevelFilter::Trace)),
            ("asdf", Err(ParseLevelError(()))),
        ];
        for &(s, ref expected) in &tests {
            assert_eq!(expected, &s.parse());
        }
    }

    #[test]
    fn test_level_from_str() {
        let tests = [
            ("OFF", Err(ParseLevelError(()))),
            ("error", Ok(Level::Error)),
            ("warn", Ok(Level::Warn)),
            ("info", Ok(Level::Info)),
            ("debug", Ok(Level::Debug)),
            ("trace", Ok(Level::Trace)),
            ("ERROR", Ok(Level::Error)),
            ("WARN", Ok(Level::Warn)),
            ("INFO", Ok(Level::Info)),
            ("DEBUG", Ok(Level::Debug)),
            ("TRACE", Ok(Level::Trace)),
            ("asdf", Err(ParseLevelError(()))),
        ];
        for &(s, ref expected) in &tests {
            assert_eq!(expected, &s.parse());
        }
    }

    #[test]
    fn test_level_as_str() {
        let tests = &[
            (Level::Error, "ERROR"),
            (Level::Warn, "WARN"),
            (Level::Info, "INFO"),
            (Level::Debug, "DEBUG"),
            (Level::Trace, "TRACE"),
        ];
        for (input, expected) in tests {
            assert_eq!(*expected, input.as_str());
        }
    }

    #[test]
    fn test_level_show() {
        assert_eq!("INFO", Level::Info.to_string());
        assert_eq!("ERROR", Level::Error.to_string());
    }

    #[test]
    fn test_levelfilter_show() {
        assert_eq!("OFF", LevelFilter::Off.to_string());
        assert_eq!("ERROR", LevelFilter::Error.to_string());
    }

    #[test]
    fn test_cross_cmp() {
        assert!(Level::Debug > LevelFilter::Error);
        assert!(LevelFilter::Warn < Level::Trace);
        assert!(LevelFilter::Off < Level::Error);
    }

    #[test]
    fn test_cross_eq() {
        assert!(Level::Error == LevelFilter::Error);
        assert!(LevelFilter::Off != Level::Error);
        assert!(Level::Trace == LevelFilter::Trace);
    }

    #[test]
    fn test_to_level() {
        assert_eq!(Some(Level::Error), LevelFilter::Error.to_level());
        assert_eq!(None, LevelFilter::Off.to_level());
        assert_eq!(Some(Level::Debug), LevelFilter::Debug.to_level());
    }

    #[test]
    fn test_to_level_filter() {
        assert_eq!(LevelFilter::Error, Level::Error.to_level_filter());
        assert_eq!(LevelFilter::Trace, Level::Trace.to_level_filter());
    }

    #[test]
    fn test_level_filter_as_str() {
        let tests = &[
            (LevelFilter::Off, "OFF"),
            (LevelFilter::Error, "ERROR"),
            (LevelFilter::Warn, "WARN"),
            (LevelFilter::Info, "INFO"),
            (LevelFilter::Debug, "DEBUG"),
            (LevelFilter::Trace, "TRACE"),
        ];
        for (input, expected) in tests {
            assert_eq!(*expected, input.as_str());
        }
    }

    #[test]
    fn test_level_up() {
        let info = Level::Info;
        let up = info.increment_severity();
        assert_eq!(up, Level::Debug);

        let trace = Level::Trace;
        let up = trace.increment_severity();
        // trace is already highest level
        assert_eq!(up, trace);
    }

    #[test]
    fn test_level_filter_up() {
        let info = LevelFilter::Info;
        let up = info.increment_severity();
        assert_eq!(up, LevelFilter::Debug);

        let trace = LevelFilter::Trace;
        let up = trace.increment_severity();
        // trace is already highest level
        assert_eq!(up, trace);
    }

    #[test]
    fn test_level_down() {
        let info = Level::Info;
        let down = info.decrement_severity();
        assert_eq!(down, Level::Warn);

        let error = Level::Error;
        let down = error.decrement_severity();
        // error is already lowest level
        assert_eq!(down, error);
    }

    #[test]
    fn test_level_filter_down() {
        let info = LevelFilter::Info;
        let down = info.decrement_severity();
        assert_eq!(down, LevelFilter::Warn);

        let error = LevelFilter::Error;
        let down = error.decrement_severity();
        assert_eq!(down, LevelFilter::Off);
        // Off is already the lowest
        assert_eq!(down.decrement_severity(), down);
    }

    #[test]
    #[cfg_attr(not(debug_assertions), ignore)]
    fn test_static_max_level_debug() {
        if cfg!(feature = "max_level_off") {
            assert_eq!(STATIC_MAX_LEVEL, LevelFilter::Off);
        } else if cfg!(feature = "max_level_error") {
            assert_eq!(STATIC_MAX_LEVEL, LevelFilter::Error);
        } else if cfg!(feature = "max_level_warn") {
            assert_eq!(STATIC_MAX_LEVEL, LevelFilter::Warn);
        } else if cfg!(feature = "max_level_info") {
            assert_eq!(STATIC_MAX_LEVEL, LevelFilter::Info);
        } else if cfg!(feature = "max_level_debug") {
            assert_eq!(STATIC_MAX_LEVEL, LevelFilter::Debug);
        } else {
            assert_eq!(STATIC_MAX_LEVEL, LevelFilter::Trace);
        }
    }

    #[test]
    #[cfg_attr(debug_assertions, ignore)]
    fn test_static_max_level_release() {
        if cfg!(feature = "release_max_level_off") {
            assert_eq!(STATIC_MAX_LEVEL, LevelFilter::Off);
        } else if cfg!(feature = "release_max_level_error") {
            assert_eq!(STATIC_MAX_LEVEL, LevelFilter::Error);
        } else if cfg!(feature = "release_max_level_warn") {
            assert_eq!(STATIC_MAX_LEVEL, LevelFilter::Warn);
        } else if cfg!(feature = "release_max_level_info") {
            assert_eq!(STATIC_MAX_LEVEL, LevelFilter::Info);
        } else if cfg!(feature = "release_max_level_debug") {
            assert_eq!(STATIC_MAX_LEVEL, LevelFilter::Debug);
        } else if cfg!(feature = "release_max_level_trace") {
            assert_eq!(STATIC_MAX_LEVEL, LevelFilter::Trace);
        } else if cfg!(feature = "max_level_off") {
            assert_eq!(STATIC_MAX_LEVEL, LevelFilter::Off);
        } else if cfg!(feature = "max_level_error") {
            assert_eq!(STATIC_MAX_LEVEL, LevelFilter::Error);
        } else if cfg!(feature = "max_level_warn") {
            assert_eq!(STATIC_MAX_LEVEL, LevelFilter::Warn);
        } else if cfg!(feature = "max_level_info") {
            assert_eq!(STATIC_MAX_LEVEL, LevelFilter::Info);
        } else if cfg!(feature = "max_level_debug") {
            assert_eq!(STATIC_MAX_LEVEL, LevelFilter::Debug);
        } else {
            assert_eq!(STATIC_MAX_LEVEL, LevelFilter::Trace);
        }
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_error_trait() {
        use super::SetLoggerError;
        let e = SetLoggerError(());
        assert_eq!(
            &e.to_string(),
            "attempted to set a logger after the logging system \
             was already initialized"
        );
    }

    #[test]
    fn test_metadata_builder() {
        use super::MetadataBuilder;
        let target = "myApp";
        let metadata_test = MetadataBuilder::new()
            .level(Level::Debug)
            .target(target)
            .build();
        assert_eq!(metadata_test.level(), Level::Debug);
        assert_eq!(metadata_test.target(), "myApp");
    }

    #[test]
    fn test_metadata_convenience_builder() {
        use super::Metadata;
        let target = "myApp";
        let metadata_test = Metadata::builder()
            .level(Level::Debug)
            .target(target)
            .build();
        assert_eq!(metadata_test.level(), Level::Debug);
        assert_eq!(metadata_test.target(), "myApp");
    }

    #[test]
    fn test_record_builder() {
        use super::{MetadataBuilder, RecordBuilder};
        let target = "myApp";
        let metadata = MetadataBuilder::new().target(target).build();
        let fmt_args = format_args!("hello");
        let record_test = RecordBuilder::new()
            .args(fmt_args)
            .metadata(metadata)
            .module_path(Some("foo"))
            .file(Some("bar"))
            .line(Some(30))
            .build();
        assert_eq!(record_test.metadata().target(), "myApp");
        assert_eq!(record_test.module_path(), Some("foo"));
        assert_eq!(record_test.file(), Some("bar"));
        assert_eq!(record_test.line(), Some(30));
    }

    #[test]
    fn test_record_convenience_builder() {
        use super::{Metadata, Record};
        let target = "myApp";
        let metadata = Metadata::builder().target(target).build();
        let fmt_args = format_args!("hello");
        let record_test = Record::builder()
            .args(fmt_args)
            .metadata(metadata)
            .module_path(Some("foo"))
            .file(Some("bar"))
            .line(Some(30))
            .build();
        assert_eq!(record_test.target(), "myApp");
        assert_eq!(record_test.module_path(), Some("foo"));
        assert_eq!(record_test.file(), Some("bar"));
        assert_eq!(record_test.line(), Some(30));
    }

    #[test]
    fn test_record_complete_builder() {
        use super::{Level, Record};
        let target = "myApp";
        let record_test = Record::builder()
            .module_path(Some("foo"))
            .file(Some("bar"))
            .line(Some(30))
            .target(target)
            .level(Level::Error)
            .build();
        assert_eq!(record_test.target(), "myApp");
        assert_eq!(record_test.level(), Level::Error);
        assert_eq!(record_test.module_path(), Some("foo"));
        assert_eq!(record_test.file(), Some("bar"));
        assert_eq!(record_test.line(), Some(30));
    }

    #[test]
    #[cfg(feature = "kv")]
    fn test_record_key_values_builder() {
        use super::Record;
        use crate::kv::{self, VisitSource};

        struct TestVisitSource {
            seen_pairs: usize,
        }

        impl<'kvs> VisitSource<'kvs> for TestVisitSource {
            fn visit_pair(
                &mut self,
                _: kv::Key<'kvs>,
                _: kv::Value<'kvs>,
            ) -> Result<(), kv::Error> {
                self.seen_pairs += 1;
                Ok(())
            }
        }

        let kvs: &[(&str, i32)] = &[("a", 1), ("b", 2)];
        let record_test = Record::builder().key_values(&kvs).build();

        let mut visitor = TestVisitSource { seen_pairs: 0 };

        record_test.key_values().visit(&mut visitor).unwrap();

        assert_eq!(2, visitor.seen_pairs);
    }

    #[test]
    #[cfg(feature = "kv")]
    fn test_record_key_values_get_coerce() {
        use super::Record;

        let kvs: &[(&str, &str)] = &[("a", "1"), ("b", "2")];
        let record = Record::builder().key_values(&kvs).build();

        assert_eq!(
            "2",
            record
                .key_values()
                .get("b".into())
                .expect("missing key")
                .to_borrowed_str()
                .expect("invalid value")
        );
    }

    // Test that the `impl Log for Foo` blocks work
    // This test mostly operates on a type level, so failures will be compile errors
    #[test]
    fn test_foreign_impl() {
        use super::Log;
        #[cfg(feature = "std")]
        use std::sync::Arc;

        fn assert_is_log<T: Log + ?Sized>() {}

        assert_is_log::<&dyn Log>();

        #[cfg(feature = "alloc")]
        assert_is_log::<Box<dyn Log>>();

        #[cfg(feature = "std")]
        assert_is_log::<Arc<dyn Log>>();

        // Assert these statements for all T: Log + ?Sized
        #[allow(unused)]
        fn forall<T: Log + ?Sized>() {
            #[cfg(feature = "alloc")]
            assert_is_log::<Box<T>>();

            assert_is_log::<&T>();

            #[cfg(feature = "std")]
            assert_is_log::<Arc<T>>();
        }
    }
}
