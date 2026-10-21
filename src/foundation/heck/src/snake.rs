use alloc::{
    borrow::ToOwned,
    fmt,
    string::{String, ToString},
};

use crate::{lowercase, transform};

/// This trait defines a snake case conversion.
///
/// In snake_case, word boundaries are indicated by underscores.
///
/// ## Example:
///
/// ```rust
/// use heck::ToSnakeCase;
///
/// let sentence = "We carry a new world here, in our hearts.";
/// assert_eq!(sentence.to_snake_case(), "we_carry_a_new_world_here_in_our_hearts");
/// ```
pub trait ToSnakeCase: ToOwned {
    /// Convert this type to snake case.
    fn to_snake_case(&self) -> Self::Owned;
}

/// Oh heck, `SnekCase` is an alias for [`ToSnakeCase`]. See ToSnakeCase for
/// more documentation.
pub trait ToSnekCase: ToOwned {
    /// Convert this type to snek case.
    fn to_snek_case(&self) -> Self::Owned;
}

impl<T: ?Sized + ToSnakeCase> ToSnekCase for T {
    fn to_snek_case(&self) -> Self::Owned {
        self.to_snake_case()
    }
}

impl ToSnakeCase for str {
    fn to_snake_case(&self) -> String {
        AsSnakeCase(self).to_string()
    }
}

/// This wrapper performs a snake case conversion in [`fmt::Display`].
///
/// ## Example:
///
/// ```
/// use heck::AsSnakeCase;
///
/// let sentence = "We carry a new world here, in our hearts.";
/// assert_eq!(format!("{}", AsSnakeCase(sentence)), "we_carry_a_new_world_here_in_our_hearts");
/// ```
pub struct AsSnakeCase<T: AsRef<str>>(pub T);

impl<T: AsRef<str>> fmt::Display for AsSnakeCase<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        transform(self.0.as_ref(), lowercase, |f| write!(f, "_"), f)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/heck/unit/src/snake/tests_63.rs"));
