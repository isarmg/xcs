//! Numeric traits and opt-in integer, iterator, complex, rational, and big-integer types.
#![no_std]
#[cfg(any(test, feature = "std"))]
#[macro_use]
extern crate std;
#[cfg(feature = "alloc")]
#[macro_use]
extern crate alloc;
pub mod traits;
pub use traits::*;
#[cfg(feature = "integer")]
pub mod integer;
#[cfg(feature = "iter")]
pub mod iter;
#[cfg(feature = "complex")]
pub mod complex;
#[cfg(feature = "bigint")]
pub mod bigint;
#[cfg(feature = "rational")]
pub mod rational;
#[cfg(feature = "collection")]
pub mod collection;
