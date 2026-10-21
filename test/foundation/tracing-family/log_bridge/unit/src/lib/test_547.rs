#[cfg(test)]
mod test {
    use super::*;

    fn test_callsite(level: log::Level) {
        let record = log::Record::builder()
            .args(format_args!("Error!"))
            .level(level)
            .target("myApp")
            .file(Some("server.rs"))
            .line(Some(144))
            .module_path(Some("server"))
            .build();

        let meta = record.as_trace();
        let (cs, _keys, _) = loglevel_to_cs(record.level());
        let cs_meta = cs.metadata();
        assert_eq!(
            meta.callsite(),
            cs_meta.callsite(),
            "actual: {:#?}\nexpected: {:#?}",
            meta,
            cs_meta
        );
        assert_eq!(meta.level(), &level.as_trace());
    }

    #[test]
    fn error_callsite_is_correct() {
        test_callsite(log::Level::Error);
    }

    #[test]
    fn warn_callsite_is_correct() {
        test_callsite(log::Level::Warn);
    }

    #[test]
    fn info_callsite_is_correct() {
        test_callsite(log::Level::Info);
    }

    #[test]
    fn debug_callsite_is_correct() {
        test_callsite(log::Level::Debug);
    }

    #[test]
    fn trace_callsite_is_correct() {
        test_callsite(log::Level::Trace);
    }
}
