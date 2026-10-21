//! Regex matchers on character and byte streams.
//!
//! ## Overview
//!
//! The [`regex`] crate implements regular expression matching on strings and byte
//! arrays. However, in order to match the output of implementations of `fmt::Debug`
//! and `fmt::Display`, or by any code which writes to an instance of `fmt::Write`
//! or `io::Write`, it is necessary to first allocate a buffer, write to that
//! buffer, and then match the buffer against a regex.
//!
//! In cases where it is not necessary to extract substrings, but only to test whether
//! or not output matches a regex, it is not strictly necessary to allocate and
//! write this output to a buffer. This crate provides a simple interface on top of
//! the lower-level [`regex-automata`] library that implements `fmt::Write` and
//! `io::Write` for regex patterns. This may be used to test whether streaming
//! output matches a pattern without buffering that output.
//!
//! Users who need to extract substrings based on a pattern or who already have
//! buffered data should probably use the [`regex`] crate instead.
//!
//! ## Syntax
//!
//! This crate uses the same [regex syntax][syntax] of the `regex-automata` crate.
//!
//! [`regex`]: https://crates.io/crates/regex
//! [`regex-automata`]: https://crates.io/crates/regex-automata
//! [syntax]: https://docs.rs/regex-automata/0.4.3/regex_automata/#syntax

use std::prelude::v1::*;
use std::{fmt, io, str::FromStr};

pub use regex::automata::dfa::dense::BuildError;
use regex::automata::dfa::dense::DFA;
use regex::automata::dfa::Automaton;
use regex::automata::util::primitives::StateID;
use regex::automata::Anchored;

/// A compiled match pattern that can match multipe inputs, or return a
/// [`Matcher`] that matches a single input.
///
/// [`Matcher`]: ../struct.Matcher.html
#[derive(Debug, Clone)]
pub struct Pattern<A = DFA<Vec<u32>>> {
    automaton: A,
    anchored: Anchored,
}

/// A reference to a [`Pattern`] that matches a single input.
///
/// [`Pattern`]: ../struct.Pattern.html
#[derive(Debug, Clone)]
pub struct Matcher<A = DFA<Vec<u32>>> {
    automaton: A,
    state: StateID,
}

// === impl Pattern ===

impl Pattern {
    /// Returns a new `Pattern` for the given regex, or an error if the regex
    /// was invalid.
    ///
    /// The returned `Pattern` will match occurances of the pattern which start
    /// at *any* in a byte or character stream — the pattern may be preceded by
    /// any number of non-matching characters. Essentially, it will behave as
    /// though the regular expression started with a `.*?`, which enables a
    /// match to appear anywhere. If this is not the desired behavior, use
    /// [`Pattern::new_anchored`] instead.
    ///
    /// For example:
    /// ```
    /// use tracing::pattern::Pattern;
    ///
    /// // This pattern matches any number of `a`s followed by a `b`.
    /// let pattern = Pattern::new("a+b").expect("regex is not invalid");
    ///
    /// // Of course, the pattern matches an input where the entire sequence of
    /// // characters matches the pattern:
    /// assert!(pattern.display_matches(&"aaaaab"));
    ///
    /// // And, since the pattern is unanchored, it will also match the
    /// // sequence when it's followed by non-matching characters:
    /// assert!(pattern.display_matches(&"hello world! aaaaab"));
    /// ```
    pub fn new(pattern: &str) -> Result<Self, BuildError> {
        let automaton = DFA::new(pattern)?;
        Ok(Pattern {
            automaton,
            anchored: Anchored::No,
        })
    }

    /// Returns a new `Pattern` anchored at the beginning of the input stream,
    /// or an error if the regex was invalid.
    ///
    /// The returned `Pattern` will *only* match an occurence of the pattern in
    /// an input sequence if the first character or byte in the input matches
    /// the pattern. If this is not the desired behavior, use [`Pattern::new`]
    /// instead.
    ///
    /// For example:
    /// ```
    /// use tracing::pattern::Pattern;
    ///
    /// // This pattern matches any number of `a`s followed by a `b`.
    /// let pattern = Pattern::new_anchored("a+b")
    ///     .expect("regex is not invalid");
    ///
    /// // The pattern matches an input where the entire sequence of
    /// // characters matches the pattern:
    /// assert!(pattern.display_matches(&"aaaaab"));
    ///
    /// // Since the pattern is anchored, it will *not* match an input that
    /// // begins with non-matching characters:
    /// assert!(!pattern.display_matches(&"hello world! aaaaab"));
    ///
    /// // ...however, if we create a pattern beginning with `.*?`, it will:
    /// let pattern2 = Pattern::new_anchored(".*?a+b")
    ///     .expect("regex is not invalid");
    /// assert!(pattern2.display_matches(&"hello world! aaaaab"));
    /// ```
    pub fn new_anchored(pattern: &str) -> Result<Self, BuildError> {
        let automaton = DFA::new(pattern)?;
        Ok(Pattern {
            automaton,
            anchored: Anchored::Yes,
        })
    }
}

impl FromStr for Pattern {
    type Err = BuildError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl<A: Automaton> Pattern<A> {
    /// Obtains a `matcher` for this pattern.
    ///
    /// This conversion is useful when wanting to incrementally feed input (via
    /// `io::Write`/`fmt::Write` to a matcher). Otherwise, the convenience methods on Pattern
    /// suffice.
    pub fn matcher(&self) -> Matcher<&'_ A> {
        let config = regex::automata::util::start::Config::new().anchored(self.anchored);
        Matcher {
            automaton: &self.automaton,
            state: self.automaton.start_state(&config).unwrap(),
        }
    }

    /// Returns `true` if this pattern matches the given string.
    #[inline]
    pub fn matches(&self, s: &impl AsRef<str>) -> bool {
        self.matcher().matches(s)
    }

    /// Returns `true` if this pattern matches the formatted output of the given
    /// type implementing `fmt::Debug`.
    ///
    /// For example:
    /// ```rust
    /// use tracing::pattern::Pattern;
    ///
    /// #[derive(Debug)]
    /// pub struct Hello {
    ///     to: &'static str,
    /// }
    ///
    /// let pattern = Pattern::new(r#"Hello \{ to: "W[^"]*" \}"#).unwrap();
    ///
    /// let hello_world = Hello { to: "World" };
    /// assert!(pattern.debug_matches(&hello_world));
    ///
    /// let hello_sf = Hello { to: "San Francisco" };
    /// assert_eq!(pattern.debug_matches(&hello_sf), false);
    ///
    /// let hello_washington = Hello { to: "Washington" };
    /// assert!(pattern.debug_matches(&hello_washington));
    /// ```
    #[inline]
    pub fn debug_matches(&self, d: &impl fmt::Debug) -> bool {
        self.matcher().debug_matches(d)
    }

    /// Returns `true` if this pattern matches the formatted output of the given
    /// type implementing `fmt::Display`.
    ///
    /// For example:
    /// ```rust
    /// # use std::fmt;
    /// use tracing::pattern::Pattern;
    ///
    /// #[derive(Debug)]
    /// pub struct Hello {
    ///     to: &'static str,
    /// }
    ///
    /// impl fmt::Display for Hello {
    ///     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    ///         write!(f, "Hello {}", self.to)
    ///     }
    /// }
    ///
    /// let pattern = Pattern::new("Hello [Ww].+").unwrap();
    ///
    /// let hello_world = Hello { to: "world" };
    /// assert!(pattern.display_matches(&hello_world));
    /// assert_eq!(pattern.debug_matches(&hello_world), false);
    ///
    /// let hello_sf = Hello { to: "San Francisco" };
    /// assert_eq!(pattern.display_matches(&hello_sf), false);
    ///
    /// let hello_washington = Hello { to: "Washington" };
    /// assert!(pattern.display_matches(&hello_washington));
    /// ```
    #[inline]
    pub fn display_matches(&self, d: &impl fmt::Display) -> bool {
        self.matcher().display_matches(d)
    }

    /// Returns either a `bool` indicating whether or not this pattern matches the
    /// data read from the provided `io::Read` stream, or an `io::Error` if an
    /// error occurred reading from the stream.
    #[inline]
    pub fn read_matches(&self, io: impl io::Read) -> io::Result<bool> {
        self.matcher().read_matches(io)
    }
}

// === impl Matcher ===

impl<A> Matcher<A>
where
    A: Automaton,
{
    #[inline]
    fn advance(&mut self, input: u8) {
        // It's safe to call `next_state_unchecked` since the matcher may
        // only be constructed by a `Pattern`, which, in turn, can only be
        // constructed with a valid DFA.
        self.state = unsafe { self.automaton.next_state_unchecked(self.state, input) };
    }

    /// Returns `true` if this `Matcher` has matched any input that has been
    /// provided.
    #[inline]
    pub fn is_matched(&self) -> bool {
        let eoi_state = self.automaton.next_eoi_state(self.state);
        self.automaton.is_match_state(eoi_state)
    }

    /// Returns `true` if this pattern matches the formatted output of the given
    /// type implementing `fmt::Debug`.
    pub fn matches(mut self, s: &impl AsRef<str>) -> bool {
        for &byte in s.as_ref().as_bytes() {
            self.advance(byte);
            if self.automaton.is_dead_state(self.state) {
                return false;
            }
        }
        self.is_matched()
    }

    /// Returns `true` if this pattern matches the formatted output of the given
    /// type implementing `fmt::Debug`.
    pub fn debug_matches(mut self, d: &impl fmt::Debug) -> bool {
        use std::fmt::Write;
        write!(&mut self, "{:?}", d).expect("matcher write impl should not fail");
        self.is_matched()
    }

    /// Returns `true` if this pattern matches the formatted output of the given
    /// type implementing `fmt::Display`.
    pub fn display_matches(mut self, d: &impl fmt::Display) -> bool {
        use std::fmt::Write;
        write!(&mut self, "{}", d).expect("matcher write impl should not fail");
        self.is_matched()
    }

    /// Returns either a `bool` indicating whether or not this pattern matches the
    /// data read from the provided `io::Read` stream, or an `io::Error` if an
    /// error occurred reading from the stream.
    pub fn read_matches(mut self, io: impl io::Read + Sized) -> io::Result<bool> {
        for r in io.bytes() {
            self.advance(r?);
            if self.automaton.is_dead_state(self.state) {
                return Ok(false);
            }
        }
        Ok(self.is_matched())
    }
}

impl<A: Automaton> fmt::Write for Matcher<A> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for &byte in s.as_bytes() {
            self.advance(byte);
            if self.automaton.is_dead_state(self.state) {
                break;
            }
        }
        Ok(())
    }
}

impl<A: Automaton> io::Write for Matcher<A> {
    fn write(&mut self, bytes: &[u8]) -> Result<usize, io::Error> {
        let mut i = 0;
        for &byte in bytes {
            self.advance(byte);
            i += 1;
            if self.automaton.is_dead_state(self.state) {
                break;
            }
        }
        Ok(i)
    }

    fn flush(&mut self) -> Result<(), io::Error> {
        Ok(())
    }
}

#[cfg(test)]
include!(core::concat!(
    core::env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/tracing-family/pattern/unit/src/lib/test_324.rs"
));
