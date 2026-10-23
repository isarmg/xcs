use core::fmt;

use alloc::{
    borrow::ToOwned,
    string::{String, ToString},
};

use crate::{capitalize, transform};

/// This trait defines a title case conversion.
///
/// In Title Case, word boundaries are indicated by spaces, and every word is
/// capitalized.
///
/// ## Example:
///
/// ```rust
/// use heck::ToTitleCase;
///
/// let sentence = "We have always lived in slums and holes in the wall.";
/// assert_eq!(sentence.to_title_case(), "We Have Always Lived In Slums And Holes In The Wall");
/// ```
pub trait ToTitleCase: ToOwned {
    /// Convert this type to title case.
    fn to_title_case(&self) -> Self::Owned;
}

impl ToTitleCase for str {
    fn to_title_case(&self) -> String {
        AsTitleCase(self).to_string()
    }
}

/// This wrapper performs a title case conversion in [`fmt::Display`].
///
/// ## Example:
///
/// ```
/// use heck::AsTitleCase;
///
/// let sentence = "We have always lived in slums and holes in the wall.";
/// assert_eq!(format!("{}", AsTitleCase(sentence)), "We Have Always Lived In Slums And Holes In The Wall");
/// ```
pub struct AsTitleCase<T: AsRef<str>>(pub T);

impl<T: AsRef<str>> fmt::Display for AsTitleCase<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        transform(self.0.as_ref(), capitalize, |f| write!(f, " "), f)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/heck/unit/src/title/tests_52.rs"));
