//! A [TOML]-compatible datetime type
//!
//! [TOML]: https://github.com/toml-lang/toml

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![warn(missing_docs)]
// Makes rustc abort compilation if there are any unsafe blocks in the crate.
// Presence of this annotation is picked up by tools such as cargo-geiger
// and lets them ensure that there is indeed no unsafe code as opposed to
// something they couldn't detect (e.g. unsafe added via macro expansion, etc).
#![forbid(unsafe_code)]
#![warn(clippy::print_stderr)]
#![warn(clippy::print_stdout)]

mod datetime;

pub use crate::datetime::datetime::Date;
pub use crate::datetime::datetime::Datetime;
pub use crate::datetime::datetime::DatetimeParseError;
pub use crate::datetime::datetime::Offset;
pub use crate::datetime::datetime::Time;

#[doc(hidden)]
#[cfg(feature = "serialization")]
pub mod __unstable {
    pub use crate::datetime::datetime::DatetimeFromString;
    pub use crate::datetime::datetime::FIELD;
    pub use crate::datetime::datetime::NAME;
}

#[doc = include_str!("../../../../../../docs/components/toml/datetime.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;
