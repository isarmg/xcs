// This file is part of ICU4X. For terms of use, please see the file
// called LICENSE at the top level of the ICU4X source tree
// (online at: https://github.com/unicode-org/icu4x/blob/main/LICENSE ).

use crate::LengthHint;

impl core::ops::Add<LengthHint> for LengthHint {
    type Output = Self;

    fn add(self, other: LengthHint) -> Self {
        LengthHint(
            self.0.saturating_add(other.0),
            match (self.1, other.1) {
                (Some(c), Some(d)) => c.checked_add(d),
                _ => None,
            },
        )
    }
}

impl core::ops::AddAssign<LengthHint> for LengthHint {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl core::iter::Sum<LengthHint> for LengthHint {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = LengthHint>,
    {
        iter.fold(LengthHint::exact(0), core::ops::Add::add)
    }
}

impl core::ops::Add<usize> for LengthHint {
    type Output = Self;

    fn add(self, other: usize) -> Self {
        Self(
            self.0.saturating_add(other),
            self.1.and_then(|upper| upper.checked_add(other)),
        )
    }
}

impl core::ops::AddAssign<usize> for LengthHint {
    fn add_assign(&mut self, other: usize) {
        *self = *self + other;
    }
}

impl core::ops::Mul<usize> for LengthHint {
    type Output = Self;

    fn mul(self, other: usize) -> Self {
        Self(
            self.0.saturating_mul(other),
            self.1.and_then(|upper| upper.checked_mul(other)),
        )
    }
}

impl core::ops::MulAssign<usize> for LengthHint {
    fn mul_assign(&mut self, other: usize) {
        *self = *self * other;
    }
}

impl core::ops::BitOr<LengthHint> for LengthHint {
    type Output = Self;

    /// Returns a new hint that is correct wherever `self` is correct, and wherever
    /// `other` is correct.
    ///
    /// Example:
    /// ```
    /// # use writeable::{LengthHint, Writeable};
    /// # use core::fmt;
    /// # fn coin_flip() -> bool { true }
    ///
    /// struct NonDeterministicWriteable(String, String);
    ///
    /// impl Writeable for NonDeterministicWriteable {
    ///     fn write_to<W: fmt::Write + ?Sized>(
    ///         &self,
    ///         sink: &mut W,
    ///     ) -> fmt::Result {
    ///         sink.write_str(if coin_flip() { &self.0 } else { &self.1 })
    ///     }
    ///
    ///     fn writeable_length_hint(&self) -> LengthHint {
    ///         LengthHint::exact(self.0.len()) | LengthHint::exact(self.1.len())
    ///     }
    /// }
    ///
    /// writeable::impl_display_with_writeable!(NonDeterministicWriteable);
    /// ```
    fn bitor(self, other: LengthHint) -> Self {
        LengthHint(
            Ord::min(self.0, other.0),
            match (self.1, other.1) {
                (Some(c), Some(d)) => Some(Ord::max(c, d)),
                _ => None,
            },
        )
    }
}

impl core::ops::BitOrAssign<LengthHint> for LengthHint {
    fn bitor_assign(&mut self, other: Self) {
        *self = *self | other;
    }
}

impl core::iter::Sum<usize> for LengthHint {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = usize>,
    {
        LengthHint::exact(iter.sum::<usize>())
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/writeable/unit/src/ops/tests_125.rs"));
