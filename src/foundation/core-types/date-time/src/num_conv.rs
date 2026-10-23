//! Private, size-checked integer conversions used by date/time arithmetic.

pub(crate) mod prelude {
    pub(crate) use super::{Truncate as _, Widen as _};
}

pub(crate) trait WidenTarget<T> {
    fn widen_into(self) -> T;
}
pub(crate) trait TruncateTarget<T> {
    fn truncate_into(self) -> T;
}

pub(crate) trait Widen: Sized {
    fn widen<T>(self) -> T
    where
        Self: WidenTarget<T>,
    {
        self.widen_into()
    }
}
impl<T> Widen for T {}

pub(crate) trait Truncate: Sized {
    fn truncate<T>(self) -> T
    where
        Self: TruncateTarget<T>,
    {
        self.truncate_into()
    }
}
impl<T> Truncate for T {}

macro_rules! impl_widen {
    ($($from:ty => $($to:ty),+;)*) => {$($(
        const _: () = assert!(core::mem::size_of::<$from>() <= core::mem::size_of::<$to>());
        impl WidenTarget<$to> for $from {
            // The same-type entries deliberately preserve a uniform conversion matrix.
            #[allow(trivial_numeric_casts)]
            #[inline]
            fn widen_into(self) -> $to { self as _ }
        }
    )+)*};
}
macro_rules! impl_truncate {
    ($($($from:ty),+ => $to:ty;)*) => {$($(
        const _: () = assert!(core::mem::size_of::<$from>() >= core::mem::size_of::<$to>());
        impl TruncateTarget<$to> for $from {
            // Identity and narrowing entries have the same low-bits conversion contract.
            #[allow(trivial_numeric_casts)]
            #[inline]
            fn truncate_into(self) -> $to { self as _ }
        }
    )+)*};
}

impl_widen! {
    u8 => u8, u16, u32, u64, u128, usize;
    u16 => u16, u32, u64, u128, usize;
    u32 => u32, u64, u128;
    u64 => u64, u128;
    u128 => u128;
    usize => usize;

    i8 => i8, i16, i32, i64, i128, isize;
    i16 => i16, i32, i64, i128, isize;
    i32 => i32, i64, i128;
    i64 => i64, i128;
    i128 => i128;
    isize => isize;
}

impl_truncate! {
    u8, u16, u32, u64, u128, usize => u8;
    u16, u32, u64, u128, usize => u16;
    u32, u64, u128 => u32;
    u64, u128 => u64;
    u128 => u128;
    usize => usize;

    i8, i16, i32, i64, i128, isize => i8;
    i16, i32, i64, i128, isize => i16;
    i32, i64, i128 => i32;
    i64, i128 => i64;
    i128 => i128;
    isize => isize;
}
