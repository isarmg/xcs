use core::fmt;

use alloc::{borrow::ToOwned, string::ToString};

use crate::{lowercase, transform};

/// This trait defines a kebab case conversion.
///
/// In kebab-case, word boundaries are indicated by hyphens.
///
/// ## Example:
///
/// ```rust
/// use heck::ToKebabCase;
///
/// let sentence = "We are going to inherit the earth.";
/// assert_eq!(sentence.to_kebab_case(), "we-are-going-to-inherit-the-earth");
/// ```
pub trait ToKebabCase: ToOwned {
    /// Convert this type to kebab case.
    fn to_kebab_case(&self) -> Self::Owned;
}

impl ToKebabCase for str {
    fn to_kebab_case(&self) -> Self::Owned {
        AsKebabCase(self).to_string()
    }
}

/// This wrapper performs a kebab case conversion in [`fmt::Display`].
///
/// ## Example:
///
/// ```
/// use heck::AsKebabCase;
///
/// let sentence = "We are going to inherit the earth.";
/// assert_eq!(format!("{}", AsKebabCase(sentence)), "we-are-going-to-inherit-the-earth");
/// ```
pub struct AsKebabCase<T: AsRef<str>>(pub T);

impl<T: AsRef<str>> fmt::Display for AsKebabCase<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        transform(self.0.as_ref(), lowercase, |f| write!(f, "-"), f)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/heck/unit/src/kebab/tests_48.rs"));
