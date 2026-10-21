//! Regressions for the build tool's timestamp-free RUST_LOG formatter.
use log::{Level, Log, Record};
use std::{
    io,
    io::Write,
    sync::{Arc, Mutex},
};

#[derive(Clone, Default)]
struct Output(Arc<Mutex<Vec<u8>>>);

impl Write for Output {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn build_logging_keeps_target_level_and_regex_filters_without_timestamps() {
    let output = Output::default();
    let logger = env_logger::Builder::new()
        .parse_filters("off,wasm_bindgen_cli_support=debug/retain")
        .write_style(env_logger::WriteStyle::Never)
        .target(env_logger::Target::Pipe(Box::new(output.clone())))
        .build();
    for (target, message, level) in [
        ("wasm_bindgen_cli_support", "retain detail", Level::Debug),
        ("wasm_bindgen_cli_support", "discard detail", Level::Debug),
        ("unrelated", "retain detail", Level::Debug),
        ("wasm_bindgen_cli_support", "retain trace", Level::Trace),
    ] {
        logger.log(
            &Record::builder()
                .target(target)
                .level(level)
                .args(format_args!("{message}"))
                .build(),
        );
    }
    assert_eq!(
        &*output.0.lock().unwrap(),
        b"[DEBUG wasm_bindgen_cli_support] retain detail\n"
    );
}

#[test]
fn explicit_color_style_remains_available() {
    let output = Output::default();
    let logger = env_logger::Builder::new()
        .parse_filters("warn")
        .write_style(env_logger::WriteStyle::Always)
        .target(env_logger::Target::Pipe(Box::new(output.clone())))
        .build();
    logger.log(
        &Record::builder()
            .target("bindings")
            .level(Level::Warn)
            .args(format_args!("diagnostic"))
            .build(),
    );
    let bytes = output.0.lock().unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("diagnostic"));
    assert!(
        text.contains("\x1b["),
        "explicit ANSI colors must remain supported: {text:?}"
    );
}
