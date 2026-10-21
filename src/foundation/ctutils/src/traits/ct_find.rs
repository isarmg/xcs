use crate::{Choice, CtAssign, CtOption};

#[cfg(doc)]
use core::iter::Iterator;

/// Constant-time equivalent of [`Iterator::find`], which can search a collection by iterating over
/// every element and applying the given predicate to each item, then selecting the first matching
/// entry.
pub trait CtFind<T: CtAssign> {
    /// Iterate through every `T` item in `&self`, applying the given `predicate` which can select
    /// a specific item by returning [`Choice::TRUE`].
    ///
    /// The first item where `predicate` returns [`Choice::TRUE`] is selected, or the [`CtOption`]
    /// equivalent of `None` is returned if the `predicate` returns [`Choice::FALSE`] for all items.
    #[must_use]
    fn ct_find<P>(&self, predicate: P) -> CtOption<T>
    where
        P: Fn(&T) -> Choice;
}

impl<T> CtFind<T> for [T]
where
    T: CtAssign + Default,
{
    #[inline]
    fn ct_find<P>(&self, predicate: P) -> CtOption<T>
    where
        P: Fn(&T) -> Choice,
    {
        let mut ret = CtOption::none();

        for item in self {
            ret.insert_if(item, predicate(item) & ret.is_none());
        }

        ret
    }
}

impl<T, const N: usize> CtFind<T> for [T; N]
where
    T: CtAssign + Default,
{
    #[inline]
    fn ct_find<P>(&self, predicate: P) -> CtOption<T>
    where
        P: Fn(&T) -> Choice,
    {
        self.as_slice().ct_find(predicate)
    }
}

#[cfg(feature = "alloc")]
mod alloc {
    use super::{Choice, CtAssign, CtFind, CtOption};
    use ::alloc::{boxed::Box, vec::Vec};

    impl<T> CtFind<T> for Box<[T]>
    where
        T: CtAssign + Default,
    {
        #[inline]
        fn ct_find<P>(&self, predicate: P) -> CtOption<T>
        where
            P: Fn(&T) -> Choice,
        {
            (**self).ct_find(predicate)
        }
    }

    #[cfg(feature = "alloc")]
    impl<T> CtFind<T> for Vec<T>
    where
        T: CtAssign + Default,
    {
        #[inline]
        fn ct_find<P>(&self, predicate: P) -> CtOption<T>
        where
            P: Fn(&T) -> Choice,
        {
            self.as_slice().ct_find(predicate)
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/ctutils/unit/src/traits/ct_find/tests_86.rs"));
