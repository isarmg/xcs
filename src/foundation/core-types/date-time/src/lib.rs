//! Project date/time capability: calendar arithmetic, UTC and fixed offsets, formatting, parsing, and serialization.
#![doc(html_playground_url = "https://play.rust-lang.org")]
#![cfg_attr(docsrs, feature(doc_cfg, doc_notable_trait))]
#![no_std]
#![doc(html_favicon_url = "https://avatars0.githubusercontent.com/u/55999857")]
#![doc(html_logo_url = "https://avatars0.githubusercontent.com/u/55999857")]
#![doc(test(attr(deny(warnings))))]

#[allow(unused_extern_crates)]
#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod calendar_core;
mod ranged;
mod display;
mod num_conv;

/// Deprecated module for units of time.
#[deprecated(note = "import from `time::unit` instead")]
pub mod convert {
    pub use crate::calendar_core::unit::*;
}
mod date;
pub mod error;
pub mod ext;
#[cfg(any(feature = "formatting", feature = "parsing"))]
pub mod format_description;
#[cfg(feature = "formatting")]
pub mod formatting;
mod hint;
#[cfg(feature = "std")]
mod instant;
mod internal_macros;
mod interop;
pub mod iter;

mod month;
mod num_fmt;
mod offset_date_time;
#[cfg(feature = "parsing")]
pub mod parsing;
mod plain_date_time;

#[cfg(feature = "serialization")]
pub mod data_serialization;
mod signed_duration;
#[cfg(test)]
#[path = "../../../../../test/foundation/date-time/modules/src/tests.rs"]
mod tests;
mod time;
mod timestamp;
mod utc_date_time;
mod utc_offset;
pub mod util;
mod weekday;

pub use crate::calendar_core::unit;

pub use crate::date::Date;
pub use crate::error::Error;
#[doc(hidden)]
#[cfg(feature = "std")]
#[expect(deprecated)]
pub use crate::instant::Instant;
pub use crate::month::Month;
pub use crate::offset_date_time::OffsetDateTime;
pub use crate::plain_date_time::PlainDateTime;
pub use crate::signed_duration::SignedDuration;
pub use crate::time::Time;
pub use crate::timestamp::Timestamp;
pub use crate::utc_date_time::UtcDateTime;
pub use crate::utc_offset::UtcOffset;
pub use crate::weekday::Weekday;

/// An alias for [`std::result::Result`] with a generic error from the time crate.
pub type Result<T> = core::result::Result<T, Error>;
/// A [`PlainDateTime`] under its original name.
///
/// This type is not currently deprecated, but it likely will be in the future. Use
/// [`PlainDateTime`] when writing new code. In the next breaking release, this alias will be
/// removed.
pub type PrimitiveDateTime = PlainDateTime;
/// A [`SignedDuration`] under its original name.
///
/// This type is not currently deprecated, but it likely will be in the future. Use
/// [`SignedDuration`] when writing new code. In the next breaking release, this alias will be
/// removed.
pub type Duration = SignedDuration;

/// A private type used to restrict the ability to call methods that are not intended to be called
/// by downstream users.
#[cfg(any(feature = "formatting", feature = "parsing"))]
#[derive(Debug)]
struct PrivateMethod;
