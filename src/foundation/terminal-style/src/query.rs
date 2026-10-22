//! Low level terminal capability lookups

/// Check [CLICOLOR] status
///
/// - When `true`, ANSI colors are supported and should be used when the program isn't piped,
///   similar to [`term_supports_color`]
/// - When `false`, don’t output ANSI color escape codes, similar to [`no_color`]
///
/// See also:
/// - [terminfo](https://crates.io/crates/terminfo) or [term](https://crates.io/crates/term) for
///   checking termcaps
/// - [termbg](https://crates.io/crates/termbg) for detecting background color
///
/// [CLICOLOR]: https://bixense.com/clicolors/
#[inline]
pub fn clicolor() -> Option<bool> {
    let value = std::env::var_os("CLICOLOR")?;
    Some(value != "0")
}

/// Check [CLICOLOR_FORCE] status
///
/// ANSI colors should be enabled no matter what.
///
/// [CLICOLOR_FORCE]: https://bixense.com/clicolors/
#[inline]
pub fn clicolor_force() -> bool {
    non_empty(std::env::var_os("CLICOLOR_FORCE").as_deref())
}

/// Check [NO_COLOR] status
///
/// When `true`, should prevent the addition of ANSI color.
///
/// User-level configuration files and per-instance command-line arguments should override
/// [NO_COLOR]. A user should be able to export `$NO_COLOR` in their shell configuration file as a
/// default, but configure a specific program in its configuration file to specifically enable
/// color.
///
/// [NO_COLOR]: https://no-color.org/
#[inline]
pub fn no_color() -> bool {
    non_empty(std::env::var_os("NO_COLOR").as_deref())
}

/// Check `TERM` for color support
#[inline]
pub fn term_supports_color() -> bool {
    {
        match std::env::var_os("TERM") {
            // If TERM isn't set, then we are in a weird environment that
            // probably doesn't support colors.
            None => return false,
            Some(k) => {
                if k == "dumb" {
                    return false;
                }
            }
        }
        true
    }
}

/// Report whether this is running in CI
///
/// CI is a common environment where, despite being piped, ansi color codes are supported
///
/// This is not as exhaustive as you'd find in a crate like `is_ci` but it should work in enough
/// cases.
#[inline]
pub fn is_ci() -> bool {
    // Assuming its CI based on presence because who would be setting `CI=false`?
    //
    // This makes it easier to all of the potential values when considering our known values:
    // - Gitlab and Github set it to `true`
    // - Woodpecker sets it to `woodpecker`
    std::env::var_os("CI").is_some()
}

fn non_empty(var: Option<&std::ffi::OsStr>) -> bool {
    !var.unwrap_or_default().is_empty()
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/terminal-style/anstyle-query/unit/src/lib/test_141.rs"
));
