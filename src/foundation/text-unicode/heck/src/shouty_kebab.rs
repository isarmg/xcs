use core::fmt;

use alloc::{borrow::ToOwned, string::ToString};

use crate::{transform, uppercase};

/// This trait defines a shouty kebab case conversion.
///
/// In SHOUTY-KEBAB-CASE, word boundaries are indicated by hyphens and all
/// words are in uppercase.
///
/// ## Example:
///
/// ```rust
/// use heck::ToShoutyKebabCase;
///
/// let sentence = "We are going to inherit the earth.";
/// assert_eq!(sentence.to_shouty_kebab_case(), "WE-ARE-GOING-TO-INHERIT-THE-EARTH");
/// ```
pub trait ToShoutyKebabCase: ToOwned {
    /// Convert this type to shouty kebab case.
    fn to_shouty_kebab_case(&self) -> Self::Owned;
}

impl ToShoutyKebabCase for str {
    fn to_shouty_kebab_case(&self) -> Self::Owned {
        AsShoutyKebabCase(self).to_string()
    }
}

/// This wrapper performs a kebab case conversion in [`fmt::Display`].
///
/// ## Example:
///
/// ```
/// use heck::AsShoutyKebabCase;
///
/// let sentence = "We are going to inherit the earth.";
/// assert_eq!(format!("{}", AsShoutyKebabCase(sentence)), "WE-ARE-GOING-TO-INHERIT-THE-EARTH");
/// ```
pub struct AsShoutyKebabCase<T: AsRef<str>>(pub T);

impl<T: AsRef<str>> fmt::Display for AsShoutyKebabCase<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        transform(self.0.as_ref(), uppercase, |f| write!(f, "-"), f)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/heck/unit/src/shouty_kebab/tests_49.rs"));
