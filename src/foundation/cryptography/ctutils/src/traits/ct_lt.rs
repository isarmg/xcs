use crate::Choice;
use core::{
    cmp,
    num::{NonZeroU8, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU128, NonZeroUsize},
};

/// Constant time less than.
pub trait CtLt {
    /// Compute whether `self < other` in constant time.
    #[must_use]
    fn ct_lt(&self, other: &Self) -> Choice;
}

// Impl `CtLt` using overflowing subtraction
macro_rules! impl_unsigned_ct_lt {
    ( $($uint:ty),+ ) => {
        $(
            impl CtLt for $uint {
                #[inline]
                fn ct_lt(&self, other: &Self) -> Choice {
                    let (_, overflow) = self.overflowing_sub(*other);
                    Choice(overflow.into())
                }
            }
        )+
    };
}

impl_unsigned_ct_lt!(u8, u16, u32, u64, u128, usize);

/// Impl `CtLt` for `NonZero<T>` by calling `NonZero::get`.
macro_rules! impl_ct_lt_for_nonzero_integer {
    ( $($ty:ty),+ ) => {
        $(
            impl CtLt for $ty {
                #[inline]
                fn ct_lt(&self, other: &Self) -> Choice {
                    self.get().ct_lt(&other.get())
                }
            }
        )+
    };
}

impl_ct_lt_for_nonzero_integer!(
    NonZeroU8,
    NonZeroU16,
    NonZeroU32,
    NonZeroU64,
    NonZeroU128,
    NonZeroUsize
);

impl CtLt for cmp::Ordering {
    #[inline]
    #[allow(clippy::arithmetic_side_effects, clippy::cast_sign_loss)]
    fn ct_lt(&self, other: &Self) -> Choice {
        let a = (*self as i8) + 1;
        let b = (*other as i8) + 1;

        // No impl of `CtLt` for `i8`, so use `u8`
        a.cast_unsigned().ct_lt(&b.cast_unsigned())
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/ctutils/unit/src/traits/ct_lt/tests_66.rs"));
