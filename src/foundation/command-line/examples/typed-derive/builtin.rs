use command_line::Args;
use command_line::builder::TypedValueParser as _;

use crate::foreign_crate;

#[derive(Args, Debug)]
pub(crate) struct BuiltInParsers {
    /// Support for discrete numbers
    #[arg(
        long,
        default_value_t = 22,
        value_parser = command_line::builder::PossibleValuesParser::new(["22", "80"])
            .map(|s| s.parse::<usize>().unwrap()),
    )]
    port: usize,

    /// Support enums from a foreign crate that don't implement `ValueEnum`
    #[arg(
        long,
        default_value_t = foreign_crate::LogLevel::Info,
        value_parser = command_line::builder::PossibleValuesParser::new(["trace", "debug", "info", "warn", "error"])
            .map(|s| s.parse::<foreign_crate::LogLevel>().unwrap()),
    )]
    log_level: foreign_crate::LogLevel,
}
