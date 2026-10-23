//! Demangle Rust compiler symbol names.
//!
//! This crate provides a `demangle` function which will return a `Demangle`
//! sentinel value that can be used to learn about the demangled version of a
//! symbol name. The demangled representation will be the same as the original
//! if it doesn't look like a mangled symbol name.
//!
//! `Demangle` can be formatted with the `Display` trait. The alternate
//! modifier (`#`) can be used to format the symbol name without the
//! trailing hash value.
//!
//! # Examples
//!
//! ```
//! use rustc_demangle::demangle;
//!
//! assert_eq!(demangle("_ZN4testE").to_string(), "test");
//! assert_eq!(demangle("_ZN3foo3barE").to_string(), "foo::bar");
//! assert_eq!(demangle("foo").to_string(), "foo");
//! // With hash
//! assert_eq!(format!("{}", demangle("_ZN3foo17h05af221e174051e9E")), "foo::h05af221e174051e9");
//! // Without hash
//! assert_eq!(format!("{:#}", demangle("_ZN3foo17h05af221e174051e9E")), "foo");
//! ```

#![no_std]
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(any(test, feature = "std"))]
#[macro_use]
extern crate std;

// HACK(eddyb) helper macros for tests.
#[cfg(test)]
macro_rules! assert_contains {
    ($s:expr, $needle:expr) => {{
        let (s, needle) = ($s, $needle);
        assert!(
            s.contains(needle),
            "{:?} should've contained {:?}",
            s,
            needle
        );
    }};
}
#[cfg(test)]
macro_rules! assert_ends_with {
    ($s:expr, $suffix:expr) => {{
        let (s, suffix) = ($s, $suffix);
        assert!(
            s.ends_with(suffix),
            "{:?} should've ended in {:?}",
            s,
            suffix
        );
    }};
}

mod legacy;
mod v0;

use core::fmt::{self, Write as _};

/// Representation of a demangled symbol name.
pub struct Demangle<'a> {
    style: Option<DemangleStyle<'a>>,
    original: &'a str,
    suffix: &'a str,
}

enum DemangleStyle<'a> {
    Legacy(legacy::Demangle<'a>),
    V0(v0::Demangle<'a>),
}

/// De-mangles a Rust symbol into a more readable version
///
/// This function will take a **mangled** symbol and return a value. When printed,
/// the de-mangled version will be written. If the symbol does not look like
/// a mangled symbol, the original value will be written instead.
///
/// # Examples
///
/// ```
/// use rustc_demangle::demangle;
///
/// assert_eq!(demangle("_ZN4testE").to_string(), "test");
/// assert_eq!(demangle("_ZN3foo3barE").to_string(), "foo::bar");
/// assert_eq!(demangle("foo").to_string(), "foo");
/// ```
pub fn demangle(mut s: &str) -> Demangle<'_> {
    // During ThinLTO LLVM may import and rename internal symbols, so strip out
    // those endings first as they're one of the last manglings applied to symbol
    // names.
    let llvm = ".llvm.";
    if let Some(i) = s.find(llvm) {
        let candidate = &s[i + llvm.len()..];
        let all_hex = candidate.chars().all(|c| match c {
            'A'..='F' | '0'..='9' | '@' => true,
            _ => false,
        });

        if all_hex {
            s = &s[..i];
        }
    }

    let mut suffix = "";
    let mut style = match legacy::demangle(s) {
        Ok((d, s)) => {
            suffix = s;
            Some(DemangleStyle::Legacy(d))
        }
        Err(()) => match v0::demangle(s) {
            Ok((d, s)) => {
                suffix = s;
                Some(DemangleStyle::V0(d))
            }
            // FIXME(eddyb) would it make sense to treat an unknown-validity
            // symbol (e.g. one that errored with `RecursedTooDeep`) as
            // v0-mangled, and have the error show up in the demangling?
            // (that error already gets past this initial check, and therefore
            // will show up in the demangling, if hidden behind a backref)
            Err(v0::ParseError::Invalid) | Err(v0::ParseError::RecursedTooDeep) => None,
        },
    };

    // Output like LLVM IR adds extra period-delimited words. See if
    // we are in that case and save the trailing words if so.
    if !suffix.is_empty() {
        if suffix.starts_with('.') && is_symbol_like(suffix) {
            // Keep the suffix.
        } else {
            // Reset the suffix and invalidate the demangling.
            suffix = "";
            style = None;
        }
    }

    Demangle {
        style,
        original: s,
        suffix,
    }
}

#[cfg(feature = "std")]
fn demangle_line(
    line: &str,
    output: &mut impl std::io::Write,
    include_hash: bool,
) -> std::io::Result<()> {
    let mut head = 0;
    while head < line.len() {
        // Move to the next potential match
        let next_head = match (line[head..].find("_ZN"), line[head..].find("_R")) {
            (Some(idx), None) | (None, Some(idx)) => head + idx,
            (Some(idx1), Some(idx2)) => head + idx1.min(idx2),
            (None, None) => {
                // No more matches...
                line.len()
            }
        };
        output.write_all(line[head..next_head].as_bytes())?;
        head = next_head;
        // Find the non-matching character.
        //
        // If we do not find a character, then until the end of the line is the
        // thing to demangle.
        let match_end = line[head..]
            .find(|ch: char| !(ch == '$' || ch == '.' || ch == '_' || ch.is_ascii_alphanumeric()))
            .map(|idx| head + idx)
            .unwrap_or(line.len());

        let mangled = &line[head..match_end];
        head = head + mangled.len();
        if let Ok(demangled) = try_demangle(mangled) {
            if include_hash {
                write!(output, "{}", demangled)?;
            } else {
                write!(output, "{:#}", demangled)?;
            }
        } else {
            output.write_all(mangled.as_bytes())?;
        }
    }
    Ok(())
}

/// Process a stream of data from `input` into the provided `output`, demangling any symbols found
/// within.
///
/// Note that the underlying implementation will perform many relatively small writes to the
/// output. If the output is expensive to write to (e.g., requires syscalls), consider using
/// `std::io::BufWriter`.
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub fn demangle_stream<R: std::io::BufRead, W: std::io::Write>(
    input: &mut R,
    output: &mut W,
    include_hash: bool,
) -> std::io::Result<()> {
    let mut buf = std::string::String::new();
    // We read in lines to reduce the memory usage at any time.
    //
    // demangle_line is also more efficient with relatively small buffers as it will copy around
    // trailing data during demangling. In the future we might directly stream to the output but at
    // least right now that seems to be less efficient.
    while input.read_line(&mut buf)? > 0 {
        demangle_line(&buf, output, include_hash)?;
        buf.clear();
    }
    Ok(())
}

/// Error returned from the `try_demangle` function below when demangling fails.
#[derive(Debug, Clone)]
pub struct TryDemangleError {
    _priv: (),
}

/// The same as `demangle`, except return an `Err` if the string does not appear
/// to be a Rust symbol, rather than "demangling" the given string as a no-op.
///
/// ```
/// extern crate rustc_demangle;
///
/// let not_a_rust_symbol = "la la la";
///
/// // The `try_demangle` function will reject strings which are not Rust symbols.
/// assert!(rustc_demangle::try_demangle(not_a_rust_symbol).is_err());
///
/// // While `demangle` will just pass the non-symbol through as a no-op.
/// assert_eq!(rustc_demangle::demangle(not_a_rust_symbol).as_str(), not_a_rust_symbol);
/// ```
pub fn try_demangle(s: &str) -> Result<Demangle<'_>, TryDemangleError> {
    let sym = demangle(s);
    if sym.style.is_some() {
        Ok(sym)
    } else {
        Err(TryDemangleError { _priv: () })
    }
}

impl<'a> Demangle<'a> {
    /// Returns the underlying string that's being demangled.
    pub fn as_str(&self) -> &'a str {
        self.original
    }
}

fn is_symbol_like(s: &str) -> bool {
    s.chars().all(|c| {
        // Once `char::is_ascii_punctuation` and `char::is_ascii_alphanumeric`
        // have been stable for long enough, use those instead for clarity
        is_ascii_alphanumeric(c) || is_ascii_punctuation(c)
    })
}

// Copied from the documentation of `char::is_ascii_alphanumeric`
fn is_ascii_alphanumeric(c: char) -> bool {
    match c {
        '\u{0041}'..='\u{005A}' | '\u{0061}'..='\u{007A}' | '\u{0030}'..='\u{0039}' => true,
        _ => false,
    }
}

// Copied from the documentation of `char::is_ascii_punctuation`
fn is_ascii_punctuation(c: char) -> bool {
    match c {
        '\u{0021}'..='\u{002F}'
        | '\u{003A}'..='\u{0040}'
        | '\u{005B}'..='\u{0060}'
        | '\u{007B}'..='\u{007E}' => true,
        _ => false,
    }
}

impl<'a> fmt::Display for DemangleStyle<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DemangleStyle::Legacy(ref d) => fmt::Display::fmt(d, f),
            DemangleStyle::V0(ref d) => fmt::Display::fmt(d, f),
        }
    }
}

// Maximum size of the symbol that we'll print.
const MAX_SIZE: usize = 1_000_000;

#[derive(Copy, Clone, Debug)]
struct SizeLimitExhausted;

struct SizeLimitedFmtAdapter<F> {
    remaining: Result<usize, SizeLimitExhausted>,
    inner: F,
}

impl<F: fmt::Write> fmt::Write for SizeLimitedFmtAdapter<F> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.remaining = self
            .remaining
            .and_then(|r| r.checked_sub(s.len()).ok_or(SizeLimitExhausted));

        match self.remaining {
            Ok(_) => self.inner.write_str(s),
            Err(SizeLimitExhausted) => Err(fmt::Error),
        }
    }
}

impl<'a> fmt::Display for Demangle<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.style {
            None => f.write_str(self.original)?,
            Some(ref d) => {
                let alternate = f.alternate();
                let mut size_limited_fmt = SizeLimitedFmtAdapter {
                    remaining: Ok(MAX_SIZE),
                    inner: &mut *f,
                };
                let fmt_result = if alternate {
                    write!(size_limited_fmt, "{:#}", d)
                } else {
                    write!(size_limited_fmt, "{}", d)
                };
                let size_limit_result = size_limited_fmt.remaining.map(|_| ());

                // Translate a `fmt::Error` generated by `SizeLimitedFmtAdapter`
                // into an error message, instead of propagating it upwards
                // (which could cause panicking from inside e.g. `std::io::print`).
                match (fmt_result, size_limit_result) {
                    (Err(_), Err(SizeLimitExhausted)) => f.write_str("{size limit reached}")?,

                    _ => {
                        fmt_result?;
                        size_limit_result
                            .expect("`fmt::Error` from `SizeLimitedFmtAdapter` was discarded");
                    }
                }
            }
        }
        f.write_str(self.suffix)
    }
}

impl<'a> fmt::Debug for Demangle<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/rustc-demangle/unit/src/lib/tests_354.rs"));
