#[allow(unused)]
use crate::Arg;

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct ArgFlags(u32);

impl ArgFlags {
    pub(crate) fn set(&mut self, setting: ArgSettings) {
        self.0 |= setting.bit();
    }

    pub(crate) fn unset(&mut self, setting: ArgSettings) {
        self.0 &= !setting.bit();
    }

    pub(crate) fn is_set(&self, setting: ArgSettings) -> bool {
        self.0 & setting.bit() != 0
    }

    pub(crate) fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

impl std::ops::BitOr for ArgFlags {
    type Output = Self;

    fn bitor(mut self, rhs: Self) -> Self::Output {
        self.insert(rhs);
        self
    }
}

/// Various settings that apply to arguments and may be set, unset, and checked via getter/setter
/// methods [`Arg::setting`], [`Arg::unset_setting`], and [`Arg::is_set`]. This is what the
/// [`Arg`] methods which accept a `bool` use internally.
///
/// [`Arg`]: command_line::Arg
/// [`Arg::setting`]: command_line::Arg::setting()
/// [`Arg::unset_setting`]: command_line::Arg::unset_setting()
/// [`Arg::is_set`]: command_line::Arg::is_set()
#[derive(Debug, PartialEq, Copy, Clone)]
#[repr(u8)]
pub(crate) enum ArgSettings {
    Required,
    Global,
    Hidden,
    NextLineHelp,
    HidePossibleValues,
    AllowHyphenValues,
    AllowNegativeNumbers,
    RequireEquals,
    Last,
    TrailingVarArg,
    HideDefaultValue,
    IgnoreCase,
    #[cfg(feature = "env")]
    HideEnv,
    #[cfg(feature = "env")]
    HideEnvValues,
    HiddenShortHelp,
    HiddenLongHelp,
    Exclusive,
}

impl ArgSettings {
    fn bit(self) -> u32 {
        1 << (self as u8)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/command-line/builder/unit/src/builder/arg_settings/test_72.rs"));
