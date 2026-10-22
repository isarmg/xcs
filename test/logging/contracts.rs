use log::{Level, LevelFilter, Log, Record};
use logging::{Builder, Target, WriteStyle};
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct Sink(Arc<Mutex<(Vec<u8>, usize)>>);
impl Write for Sink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.lock().unwrap().1 += 1;
        Ok(())
    }
}
fn output(style: WriteStyle) -> (Vec<u8>, usize) {
    let sink = Sink::default();
    let logger = Builder::new()
        .filter_level(LevelFilter::Info)
        .target(Target::Pipe(Box::new(sink.clone())))
        .write_style(style)
        .format(|buf, record| writeln!(buf, "\x1b[31m{}\x1b[0m", record.args()))
        .build();
    logger.log(
        &Record::builder()
            .level(Level::Debug)
            .args(format_args!("hidden"))
            .build(),
    );
    logger.log(
        &Record::builder()
            .level(Level::Info)
            .args(format_args!("visible"))
            .build(),
    );
    let result = sink.0.lock().unwrap().clone();
    result
}
#[test]
fn pipe_filtering_and_flushing_preserve_output() {
    let (bytes, flushes) = output(WriteStyle::Always);
    assert_eq!(bytes, b"\x1b[31mvisible\x1b[0m\n");
    assert_eq!(flushes, 1);
}
#[test]
fn color_adaptation_is_feature_accurate() {
    let (bytes, _) = output(WriteStyle::Never);
    #[cfg(feature = "color")]
    assert_eq!(bytes, b"visible\n");
    #[cfg(not(feature = "color"))]
    assert_eq!(bytes, b"\x1b[31mvisible\x1b[0m\n");
}
#[test]
fn target_precedence_and_message_match() {
    let logger = Builder::new()
        .parse_filters("error,app=info,app::detail=debug/foo.*bar")
        .build();
    let matches = |target: &str, level, text: &str| {
        logger.matches(
            &Record::builder()
                .target(target)
                .level(level)
                .args(format_args!("{text}"))
                .build(),
        )
    };
    assert!(!matches("app", Level::Debug, "foo.*bar"));
    assert!(matches("app::detail", Level::Debug, "foo.*bar"));
    assert!(!matches("app::detail", Level::Debug, "unrelated"));
    assert!(!matches("other", Level::Warn, "foo.*bar"));
    assert_eq!(
        matches("app", Level::Info, "fooXYZbar"),
        cfg!(feature = "regex")
    );
}
#[test]
fn strict_parse_rejects_invalid_levels_and_default_is_error() {
    assert!(logging::filter::Builder::new()
        .try_parse("app=impossible")
        .is_err());
    assert_eq!(Builder::new().build().filter(), LevelFilter::Error);
}
#[test]
fn default_format_keeps_level_target_and_newline() {
    let sink = Sink::default();
    let logger = Builder::new()
        .filter_level(LevelFilter::Info)
        .target(Target::Pipe(Box::new(sink.clone())))
        .write_style(WriteStyle::Never)
        .build();
    logger.log(
        &Record::builder()
            .target("app")
            .level(Level::Info)
            .args(format_args!("hello"))
            .build(),
    );
    assert_eq!(sink.0.lock().unwrap().0, b"[INFO  app] hello\n");
}
