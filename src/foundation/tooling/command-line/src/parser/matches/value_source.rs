/// Origin of the argument's value
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum ValueSource {
    /// Value came [`Arg::default_value`][command_line::Arg::default_value]
    DefaultValue,
    /// Value came [`Arg::env`][command_line::Arg::env]
    EnvVariable,
    /// Value was passed in on the command-line
    CommandLine,
}

impl ValueSource {
    pub(crate) fn is_explicit(self) -> bool {
        self != Self::DefaultValue
    }
}
