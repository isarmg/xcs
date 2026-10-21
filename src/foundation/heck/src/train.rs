use core::fmt;

use alloc::{borrow::ToOwned, string::ToString};

use crate::{capitalize, transform};

/// This trait defines a train case conversion.
///
/// In Train-Case, word boundaries are indicated by hyphens and words start
/// with Capital Letters.
///
/// ## Example:
///
/// ```rust
/// use heck::ToTrainCase;
///
/// let sentence = "We are going to inherit the earth.";
/// assert_eq!(sentence.to_train_case(), "We-Are-Going-To-Inherit-The-Earth");
/// ```
pub trait ToTrainCase: ToOwned {
    /// Convert this type to Train-Case.
    fn to_train_case(&self) -> Self::Owned;
}

impl ToTrainCase for str {
    fn to_train_case(&self) -> Self::Owned {
        AsTrainCase(self).to_string()
    }
}

/// This wrapper performs a train case conversion in [`fmt::Display`].
///
/// ## Example:
///
/// ```
/// use heck::AsTrainCase;
///
/// let sentence = "We are going to inherit the earth.";
/// assert_eq!(format!("{}", AsTrainCase(sentence)), "We-Are-Going-To-Inherit-The-Earth");
/// ```
pub struct AsTrainCase<T: AsRef<str>>(pub T);

impl<T: AsRef<str>> fmt::Display for AsTrainCase<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        transform(self.0.as_ref(), capitalize, |f| write!(f, "-"), f)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/heck/unit/src/train/tests_49.rs"));
