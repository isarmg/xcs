//! Shared ANSI styles, optional no_std parser, and explicit native output.
#![cfg_attr(all(not(feature = "std"), not(test)), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
#![warn(clippy::std_instead_of_core)]
#![warn(clippy::std_instead_of_alloc)]
#![warn(clippy::print_stderr)]
#![warn(clippy::print_stdout)]

#[macro_use]
mod macros;

mod color;
mod effect;
mod reset;
mod style;

pub use color::*;
pub use effect::*;
pub use reset::*;
pub use style::*;

mod choice;
pub use choice::ColorChoice;
#[cfg(feature = "output")]
pub mod output;
#[cfg(feature = "parser")]
pub mod parser;
#[cfg(feature = "output")]
mod query;
