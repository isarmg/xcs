#![cfg_attr(not(feature = "usage"), allow(dead_code))]
use std::borrow::Cow;

/// Terminal-styling container
///
/// Styling may be encoded as [ANSI Escape Code](https://en.wikipedia.org/wiki/ANSI_escape_code)
///
/// # Examples
///
/// ```rust
/// # use command_line;
/// // ANSI escapes encode styling without an additional formatting dependency.
/// let after_help: &'static str = "\x1b[1;4mExamples\x1b[0m\n\n\x20\x20\x1b[2m$\x1b[0m \
///     \x1b[1mmybin --input file.toml\x1b[0m\n";
///
/// let cmd = command_line::Command::new("mybin")
///     .after_help(after_help)  // The `&str` gets converted into a `StyledStr`
///     // ...
/// #   ;
/// ```
#[derive(Clone, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StyledStr(String);

impl StyledStr {
    /// Create an empty buffer
    pub const fn new() -> Self {
        Self(String::new())
    }

    /// Display using [ANSI Escape Code](https://en.wikipedia.org/wiki/ANSI_escape_code) styling
    #[cfg(feature = "color")]
    pub fn ansi(&self) -> impl std::fmt::Display + '_ {
        self.0.as_str()
    }

    /// May allow the compiler to consolidate the `Drop`s for `msg`, reducing code size compared to
    /// `styled.push_str(&msg)`
    pub(crate) fn push_string(&mut self, msg: String) {
        self.0.push_str(&msg);
    }

    /// Appends a given string slice onto the end of this `StyledStr`.
    pub fn push_str(&mut self, msg: &str) {
        self.0.push_str(msg);
    }

    pub(crate) fn trim_start_lines(&mut self) {
        if let Some(pos) = self.0.find('\n') {
            let (leading, help) = self.0.split_at(pos + 1);
            if leading.trim().is_empty() {
                self.0 = help.to_owned();
            }
        }
    }

    pub(crate) fn trim_end(&mut self) {
        self.0 = self.0.trim_end().to_owned();
    }

    #[cfg(feature = "help")]
    pub(crate) fn replace_newline_var(&mut self) {
        self.0 = self.0.replace("{n}", "\n");
    }

    #[cfg(feature = "help")]
    pub(crate) fn indent(&mut self, initial: &str, trailing: &str) {
        self.0.insert_str(0, initial);

        let mut line_sep = "\n".to_owned();
        line_sep.push_str(trailing);
        self.0 = self.0.replace('\n', &line_sep);
    }

    #[cfg(feature = "help")]
    pub(crate) fn wrap(&mut self, _hard_width: usize) {}



    #[inline(never)]
    #[cfg(feature = "help")]
    pub(crate) fn display_width(&self) -> usize {
        let mut width = 0;
        for c in self.iter_text() {
            width += crate::output::display_width(c);
        }
        width
    }

    #[cfg(feature = "help")]
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    #[cfg(feature = "help")]
    pub(crate) fn as_styled_str(&self) -> &str {
        &self.0
    }

    #[cfg(feature = "color")]
    pub(crate) fn iter_text(&self) -> impl Iterator<Item = &str> {
        terminal_style::output::adapter::strip_str(&self.0)
    }

    #[cfg(not(feature = "color"))]
    pub(crate) fn iter_text(&self) -> impl Iterator<Item = &str> {
        [self.0.as_str()].into_iter()
    }

    pub(crate) fn push_styled(&mut self, other: &Self) {
        self.0.push_str(&other.0);
    }

    pub(crate) fn write_to(&self, buffer: &mut dyn std::io::Write) -> std::io::Result<()> {
        ok!(buffer.write_all(self.0.as_bytes()));

        Ok(())
    }
}

impl Default for &'_ StyledStr {
    fn default() -> Self {
        static DEFAULT: StyledStr = StyledStr::new();
        &DEFAULT
    }
}

impl From<String> for StyledStr {
    fn from(name: String) -> Self {
        Self(name)
    }
}

impl From<&'_ String> for StyledStr {
    fn from(name: &'_ String) -> Self {
        let mut styled = Self::new();
        styled.push_str(name);
        styled
    }
}

impl From<&'static str> for StyledStr {
    fn from(name: &'static str) -> Self {
        let mut styled = Self::new();
        styled.push_str(name);
        styled
    }
}

impl From<&'_ &'static str> for StyledStr {
    fn from(name: &'_ &'static str) -> Self {
        Self::from(*name)
    }
}

impl From<Cow<'static, str>> for StyledStr {
    fn from(cow: Cow<'static, str>) -> Self {
        match cow {
            Cow::Borrowed(s) => Self::from(s),
            Cow::Owned(s) => Self::from(s),
        }
    }
}

impl std::fmt::Write for StyledStr {
    #[inline]
    fn write_str(&mut self, s: &str) -> Result<(), std::fmt::Error> {
        self.0.push_str(s);
        Ok(())
    }

    #[inline]
    fn write_char(&mut self, c: char) -> Result<(), std::fmt::Error> {
        self.0.push(c);
        Ok(())
    }
}

/// Color-unaware printing. Never uses coloring.
impl std::fmt::Display for StyledStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for part in self.iter_text() {
            part.fmt(f)?;
        }

        Ok(())
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/command-line/builder/unit/src/builder/styled_str/wrap_tests_223.rs"));

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/command-line/builder/unit/src/builder/styled_str/tests_270.rs"));
