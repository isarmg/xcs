use crate::{CtAssign, CtEq, CtOption};
use core::ops::AddAssign;

#[cfg(doc)]
use core::ops::Index;

/// Constant-time lookup by index, similar to the [`Index`] trait, but returning an owned result in
/// constant-time.
pub trait CtLookup<Idx> {
    /// Output type returned by the lookup operation.
    type Output: CtAssign;

    /// Attempt to retrieve the item at the given `index`, either returning it or the [`CtOption`]
    /// equivalent of [`None`] if the `index` was out-of-bounds.
    #[must_use]
    fn ct_lookup(&self, index: Idx) -> CtOption<Self::Output>;
}

impl<T, Idx> CtLookup<Idx> for [T]
where
    T: CtAssign + Default,
    Idx: AddAssign + CtEq + Default + From<u8>,
{
    type Output = T;

    #[inline]
    #[allow(clippy::arithmetic_side_effects)]
    fn ct_lookup(&self, index: Idx) -> CtOption<T> {
        let mut ret = CtOption::none();
        let mut i = Idx::default();

        for item in self {
            ret.insert_if(item, i.ct_eq(&index));

            // TODO(tarcieri): ideally we'd prevent overflow here but there's no core `CheckedAdd`
            i += Idx::from(1u8);
        }

        ret
    }
}

impl<T, Idx, const N: usize> CtLookup<Idx> for [T; N]
where
    T: CtAssign + Default,
    Idx: AddAssign + CtEq + Default + From<u8>,
{
    type Output = T;

    #[inline]
    fn ct_lookup(&self, index: Idx) -> CtOption<T> {
        self.as_slice().ct_lookup(index)
    }
}

#[cfg(feature = "alloc")]
mod alloc {
    use super::{AddAssign, CtAssign, CtEq, CtLookup, CtOption};
    use ::alloc::{boxed::Box, vec::Vec};

    impl<T, Idx> CtLookup<Idx> for Box<[T]>
    where
        T: CtAssign + Default,
        Idx: AddAssign + CtEq + Default + From<u8>,
    {
        type Output = T;

        #[inline]
        fn ct_lookup(&self, index: Idx) -> CtOption<T> {
            (**self).ct_lookup(index)
        }
    }

    impl<T, Idx> CtLookup<Idx> for Vec<T>
    where
        T: CtAssign + Default,
        Idx: AddAssign + CtEq + Default + From<u8>,
    {
        type Output = T;

        #[inline]
        fn ct_lookup(&self, index: Idx) -> CtOption<T> {
            self.as_slice().ct_lookup(index)
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/ctutils/unit/src/traits/ct_lookup/tests_88.rs"));
