// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

use crate::Writeable;
use core::cmp::Ordering;
use core::fmt;

struct WriteComparator<'a> {
    code_units: &'a [u8],
    result: Ordering,
}

/// This is an infallible impl. Functions always return Ok, not Err.
impl fmt::Write for WriteComparator<'_> {
    #[inline]
    fn write_str(&mut self, other: &str) -> fmt::Result {
        if self.result != Ordering::Equal {
            return Ok(());
        }
        let (this, remainder) = self
            .code_units
            .split_at_checked(other.len())
            .unwrap_or((self.code_units, &[]));
        self.code_units = remainder;
        self.result = this.cmp(other.as_bytes());
        Ok(())
    }
}

impl<'a> WriteComparator<'a> {
    #[inline]
    fn new(code_units: &'a [u8]) -> Self {
        Self {
            code_units,
            result: Ordering::Equal,
        }
    }

    #[inline]
    fn finish(self) -> Ordering {
        if matches!(self.result, Ordering::Equal) && !self.code_units.is_empty() {
            // Self is longer than Other
            Ordering::Greater
        } else {
            self.result
        }
    }
}

/// Compares the contents of a [`Writeable`] to the given UTF-8 bytes without allocating memory.
///
/// For more details, see: [`cmp_str`]
pub fn cmp_utf8(writeable: &impl Writeable, other: &[u8]) -> Ordering {
    let mut wc = WriteComparator::new(other);
    let _ = writeable.write_to(&mut wc);
    wc.finish().reverse()
}

/// Compares the contents of a `Writeable` to the given bytes
/// without allocating a String to hold the `Writeable` contents.
///
/// This returns a lexicographical comparison, the same as if the Writeable
/// were first converted to a String and then compared with `Ord`. For a
/// string ordering suitable for display to end users, use a localized
/// collation crate, such as `icu_collator`.
///
/// # Examples
///
/// ```
/// use core::cmp::Ordering;
/// use core::fmt;
/// use writeable::Writeable;
///
/// struct WelcomeMessage<'s> {
///     pub name: &'s str,
/// }
///
/// impl<'s> Writeable for WelcomeMessage<'s> {
///     // see impl in Writeable docs
/// #    fn write_to<W: fmt::Write + ?Sized>(&self, sink: &mut W) -> fmt::Result {
/// #        sink.write_str("Hello, ")?;
/// #        sink.write_str(self.name)?;
/// #        sink.write_char('!')?;
/// #        Ok(())
/// #    }
/// }
///
/// let message = WelcomeMessage { name: "Alice" };
/// let message_str = message.write_to_string();
///
/// assert_eq!(Ordering::Equal, writeable::cmp_str(&message, "Hello, Alice!"));
///
/// assert_eq!(Ordering::Greater, writeable::cmp_str(&message, "Alice!"));
/// assert_eq!(Ordering::Greater, (*message_str).cmp("Alice!"));
///
/// assert_eq!(Ordering::Less, writeable::cmp_str(&message, "Hello, Bob!"));
/// assert_eq!(Ordering::Less, (*message_str).cmp("Hello, Bob!"));
/// ```
///
/// This function can be combined with `writeable::concat_writeable!` to make an efficient
/// comparison between a string and a sequence of substrings:
///
/// ```
/// use core::cmp::Ordering;
///
/// assert_eq!(
///     Ordering::Less,
///     writeable::cmp_str(&writeable::concat_writeable!("loop", 1), "loop12")
/// );
/// assert_eq!(
///     Ordering::Equal,
///     writeable::cmp_str(&writeable::concat_writeable!("loop", 12), "loop12")
/// );
/// assert_eq!(
///     Ordering::Greater,
///     writeable::cmp_str(&writeable::concat_writeable!("loop", 2), "loop12")
/// );
/// ```
#[inline]
pub fn cmp_str(writeable: &impl Writeable, other: &str) -> Ordering {
    cmp_utf8(writeable, other.as_bytes())
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/writeable/unit/src/cmp/tests_125.rs"));
