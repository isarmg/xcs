#[test]
fn cli_and_logger_share_terminal_styles() {
    use std::io::Write;
    use std::sync::{Arc, Mutex};
    let green = terminal_style::AnsiColor::Green.on_default();
    let styles = command_line::builder::Styles::styled().header(green);
    let mut cmd = command_line::Command::new("demo").styles(styles).arg(
        command_line::Arg::new("verbose")
            .long("verbose")
            .action(command_line::ArgAction::SetTrue),
    );
    assert!(cmd.render_help().to_string().contains("--verbose"));
    #[derive(Clone)]
    struct Sink(Arc<Mutex<Vec<u8>>>);
    impl Write for Sink {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let bytes = Arc::new(Mutex::new(Vec::new()));
    let mut logger = env_logger::Builder::new();
    logger
        .target(env_logger::Target::Pipe(Box::new(Sink(bytes.clone()))))
        .filter_level(log::LevelFilter::Info)
        .write_style(env_logger::WriteStyle::Never);
    logger.format(move |buf, record| writeln!(buf, "{green}{}{green:#}", record.args()));
    let logger = logger.build();
    log::Log::log(
        &logger,
        &log::Record::builder()
            .args(format_args!("message"))
            .level(log::Level::Info)
            .build(),
    );
    assert_eq!(&*bytes.lock().unwrap(), b"message\n");
}
