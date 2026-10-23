/// Reset terminal formatting
#[allow(clippy::exhaustive_structs)]
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Reset;

impl Reset {
    /// Render the ANSI code
    ///
    /// `Reset` also implements `Display` directly, so calling this method is optional.
    #[inline]
    pub fn render(self) -> impl core::fmt::Display + Copy {
        self
    }
}

impl core::fmt::Display for Reset {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(RESET)
    }
}

pub(crate) const RESET: &str = "\x1B[0m";

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../../test/foundation/terminal-style/anstyle/unit/src/reset/test_24.rs"
));
