#[cfg(any(feature = "std", feature = "alloc"))]
mod content;
mod seed;

// FIXME: #[cfg(doctest)] once https://github.com/rust-lang/rust/issues/67295 is fixed.
#[doc(hidden)]
pub mod doc;

#[doc(hidden)]
pub mod size_hint;

#[doc(hidden)]
pub mod string;

#[cfg(any(feature = "std", feature = "alloc"))]
#[doc(hidden)]
pub use self::content::Content;
#[doc(hidden)]
pub use self::seed::InPlaceSeed;
#[doc(hidden)]
#[cfg(feature = "derive")]
pub mod de;

#[cfg(feature = "derive")]
pub mod ser;

#[cfg(feature = "derive")]
pub use crate::lib::clone::Clone;
#[cfg(feature = "derive")]
pub use crate::lib::convert::{From, Into, TryFrom};
#[cfg(feature = "derive")]
pub use crate::lib::default::Default;
#[cfg(feature = "derive")]
pub use crate::lib::fmt::{self, Formatter};
#[cfg(feature = "derive")]
pub use crate::lib::marker::PhantomData;
#[cfg(feature = "derive")]
pub use crate::lib::option::Option::{self, None, Some};
#[cfg(feature = "derive")]
pub use crate::lib::ptr;
#[cfg(feature = "derive")]
pub use crate::lib::result::Result::{self, Err, Ok};

#[cfg(feature = "derive")]
pub use self::string::from_utf8_lossy;

#[cfg(any(feature = "alloc", feature = "std"))]
#[cfg(feature = "derive")]
pub use crate::lib::{ToString, Vec};
