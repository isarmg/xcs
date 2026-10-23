//! Calendar utility functions.

use core::num::NonZero;

pub(crate) use crate::calendar_core::util::{days_in_month_leap, range_validated};
pub use crate::calendar_core::util::{days_in_year, is_leap_year, weeks_in_year};

use crate::Month;

/// Which direction arithmetic overflow occurred in.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Overflow {
    /// The overflow was positive (i.e. towards positive infinity).
    Positive,
    /// The overflow was negative (i.e. towards negative infinity).
    Negative,
}

/// Whether to adjust the date, and in which direction. Useful when implementing arithmetic.
pub(crate) enum DateAdjustment {
    /// The previous day should be used.
    Previous,
    /// The next day should be used.
    Next,
    /// The date should be used as-is.
    None,
}

/// Get the number of days in the month of a given year.
///
/// ```rust
/// # use time::{Month, util};
/// assert_eq!(util::days_in_month(Month::February, 2020), 29);
/// ```
#[inline]
pub const fn days_in_month(month: Month, year: i32) -> u8 {
    crate::calendar_core::util::days_in_month(month as u8, year)
}

/// Get the number of days in the month of a given year.
///
/// ```rust
/// # #![expect(deprecated)]
/// # use time::{Month, util};
/// assert_eq!(util::days_in_year_month(2020, Month::February), 29);
/// ```
#[deprecated(
    since = "0.3.37",
    note = "use `days_in_month` or `Month::length` instead"
)]
#[inline]
pub const fn days_in_year_month(year: i32, month: Month) -> u8 {
    days_in_month(month, year)
}

/// Given whether a year is a leap year and the ordinal day, return the month and day of the month.
#[inline]
pub(crate) const fn leap_ordinal_to_month_day(leap: bool, ordinal: u16) -> (Month, u8) {
    let ordinal = ordinal as u32;
    let jan_feb_len = 59 + leap as u32;

    let (month_adj, ordinal_adj) = if ordinal <= jan_feb_len {
        (0, 0)
    } else {
        (2, jan_feb_len)
    };

    let ordinal = ordinal - ordinal_adj;
    let month = (ordinal * 268 + 8031) >> 13;
    let days_in_preceding_months = (month * 3917 - 3866) >> 7;
    let day = ordinal - days_in_preceding_months;
    let month = month + month_adj;

    (
        // Safety: `month` is guaranteed to be between 1 and 12 inclusive.
        unsafe {
            match Month::from_number(NonZero::new_unchecked(month as u8)) {
                Ok(month) => month,
                Err(_) => core::hint::unreachable_unchecked(),
            }
        },
        day as u8,
    )
}
