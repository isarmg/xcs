use core::fmt;

use alloc::{borrow::ToOwned, string::ToString};

use crate::{transform, uppercase};

/// This trait defines a shouty snake case conversion.
///
/// In SHOUTY_SNAKE_CASE, word boundaries are indicated by underscores and all
/// words are in uppercase.
///
/// ## Example:
///
/// ```rust
/// use heck::ToShoutySnakeCase;
///
/// let sentence = "That world is growing in this minute.";
/// assert_eq!(sentence.to_shouty_snake_case(), "THAT_WORLD_IS_GROWING_IN_THIS_MINUTE");
/// ```
pub trait ToShoutySnakeCase: ToOwned {
    /// Convert this type to shouty snake case.
    fn to_shouty_snake_case(&self) -> Self::Owned;
}

/// Oh heck, `ToShoutySnekCase` is an alias for [`ToShoutySnakeCase`]. See
/// ToShoutySnakeCase for more documentation.
pub trait ToShoutySnekCase: ToOwned {
    /// CONVERT THIS TYPE TO SNEK CASE.
    #[allow(non_snake_case)]
    fn TO_SHOUTY_SNEK_CASE(&self) -> Self::Owned;
}

impl<T: ?Sized + ToShoutySnakeCase> ToShoutySnekCase for T {
    fn TO_SHOUTY_SNEK_CASE(&self) -> Self::Owned {
        self.to_shouty_snake_case()
    }
}

impl ToShoutySnakeCase for str {
    fn to_shouty_snake_case(&self) -> Self::Owned {
        AsShoutySnakeCase(self).to_string()
    }
}

/// This wrapper performs a shouty snake  case conversion in [`fmt::Display`].
///
/// ## Example:
///
/// ```
/// use heck::AsShoutySnakeCase;
///
/// let sentence = "That world is growing in this minute.";
/// assert_eq!(format!("{}", AsShoutySnakeCase(sentence)), "THAT_WORLD_IS_GROWING_IN_THIS_MINUTE");
/// ```
pub struct AsShoutySnakeCase<T: AsRef<str>>(pub T);

impl<T: AsRef<str>> fmt::Display for AsShoutySnakeCase<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        transform(self.0.as_ref(), uppercase, |f| write!(f, "_"), f)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/heck/unit/src/shouty_snake/tests_63.rs"));
