//! Rayon extensions for `HashSet`.

use super::map;
use crate::HashSet;
use crate::alloc::{Allocator, Global};
use core::hash::{BuildHasher, Hash};
use rayon::iter::plumbing::UnindexedConsumer;
use rayon::iter::{FromParallelIterator, IntoParallelIterator, ParallelExtend, ParallelIterator};

/// Parallel iterator over elements of a consumed set.
///
/// This iterator is created by the [`into_par_iter`] method on [`HashSet`]
/// (provided by the [`IntoParallelIterator`] trait).
/// See its documentation for more.
///
/// [`into_par_iter`]: rayon::iter::IntoParallelIterator::into_par_iter
/// [`IntoParallelIterator`]: rayon::iter::IntoParallelIterator
pub struct IntoParIter<T, A: Allocator = Global> {
    inner: map::IntoParIter<T, (), A>,
}

impl<T: Send, A: Allocator + Send> ParallelIterator for IntoParIter<T, A> {
    type Item = T;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        self.inner.map(|(k, _)| k).drive_unindexed(consumer)
    }
}

/// Parallel draining iterator over entries of a set.
///
/// This iterator is created by the [`par_drain`] method on [`HashSet`].
/// See its documentation for more.
///
/// [`par_drain`]: HashSet::par_drain
pub struct ParDrain<'a, T, A: Allocator = Global> {
    inner: map::ParDrain<'a, T, (), A>,
}

impl<T: Send, A: Allocator + Send + Sync> ParallelIterator for ParDrain<'_, T, A> {
    type Item = T;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        self.inner.map(|(k, _)| k).drive_unindexed(consumer)
    }
}

/// Parallel iterator over shared references to elements in a set.
///
/// This iterator is created by the [`par_iter`] method on [`HashSet`]
/// (provided by the [`IntoParallelRefIterator`] trait).
/// See its documentation for more.
///
/// [`par_iter`]: rayon::iter::IntoParallelRefIterator::par_iter
/// [`IntoParallelRefIterator`]: rayon::iter::IntoParallelRefIterator
pub struct ParIter<'a, T> {
    inner: map::ParKeys<'a, T, ()>,
}

impl<'a, T: Sync> ParallelIterator for ParIter<'a, T> {
    type Item = &'a T;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        self.inner.drive_unindexed(consumer)
    }
}

/// Parallel iterator over shared references to elements in the difference of
/// sets.
///
/// This iterator is created by the [`par_difference`] method on [`HashSet`].
/// See its documentation for more.
///
/// [`par_difference`]: HashSet::par_difference
pub struct ParDifference<'a, T, S, A: Allocator = Global> {
    a: &'a HashSet<T, S, A>,
    b: &'a HashSet<T, S, A>,
}

impl<'a, T, S, A> ParallelIterator for ParDifference<'a, T, S, A>
where
    T: Eq + Hash + Sync,
    S: BuildHasher + Sync,
    A: Allocator + Sync,
{
    type Item = &'a T;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        self.a
            .into_par_iter()
            .filter(|&x| !self.b.contains(x))
            .drive_unindexed(consumer)
    }
}

/// Parallel iterator over shared references to elements in the symmetric
/// difference of sets.
///
/// This iterator is created by the [`par_symmetric_difference`] method on
/// [`HashSet`].
/// See its documentation for more.
///
/// [`par_symmetric_difference`]: HashSet::par_symmetric_difference
pub struct ParSymmetricDifference<'a, T, S, A: Allocator = Global> {
    a: &'a HashSet<T, S, A>,
    b: &'a HashSet<T, S, A>,
}

impl<'a, T, S, A> ParallelIterator for ParSymmetricDifference<'a, T, S, A>
where
    T: Eq + Hash + Sync,
    S: BuildHasher + Sync,
    A: Allocator + Sync,
{
    type Item = &'a T;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        self.a
            .par_difference(self.b)
            .chain(self.b.par_difference(self.a))
            .drive_unindexed(consumer)
    }
}

/// Parallel iterator over shared references to elements in the intersection of
/// sets.
///
/// This iterator is created by the [`par_intersection`] method on [`HashSet`].
/// See its documentation for more.
///
/// [`par_intersection`]: HashSet::par_intersection
pub struct ParIntersection<'a, T, S, A: Allocator = Global> {
    a: &'a HashSet<T, S, A>,
    b: &'a HashSet<T, S, A>,
}

impl<'a, T, S, A> ParallelIterator for ParIntersection<'a, T, S, A>
where
    T: Eq + Hash + Sync,
    S: BuildHasher + Sync,
    A: Allocator + Sync,
{
    type Item = &'a T;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        self.a
            .into_par_iter()
            .filter(|&x| self.b.contains(x))
            .drive_unindexed(consumer)
    }
}

/// Parallel iterator over shared references to elements in the union of sets.
///
/// This iterator is created by the [`par_union`] method on [`HashSet`].
/// See its documentation for more.
///
/// [`par_union`]: HashSet::par_union
pub struct ParUnion<'a, T, S, A: Allocator = Global> {
    a: &'a HashSet<T, S, A>,
    b: &'a HashSet<T, S, A>,
}

impl<'a, T, S, A> ParallelIterator for ParUnion<'a, T, S, A>
where
    T: Eq + Hash + Sync,
    S: BuildHasher + Sync,
    A: Allocator + Sync,
{
    type Item = &'a T;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        // We'll iterate one set in full, and only the remaining difference from the other.
        // Use the smaller set for the difference in order to reduce hash lookups.
        let (smaller, larger) = if self.a.len() <= self.b.len() {
            (self.a, self.b)
        } else {
            (self.b, self.a)
        };
        larger
            .into_par_iter()
            .chain(smaller.par_difference(larger))
            .drive_unindexed(consumer)
    }
}

impl<T, S, A> HashSet<T, S, A>
where
    T: Eq + Hash + Sync,
    S: BuildHasher + Sync,
    A: Allocator + Sync,
{
    /// Visits (potentially in parallel) the values representing the union,
    /// i.e. all the values in `self` or `other`, without duplicates.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn par_union<'a>(&'a self, other: &'a Self) -> ParUnion<'a, T, S, A> {
        ParUnion { a: self, b: other }
    }

    /// Visits (potentially in parallel) the values representing the difference,
    /// i.e. the values that are in `self` but not in `other`.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn par_difference<'a>(&'a self, other: &'a Self) -> ParDifference<'a, T, S, A> {
        ParDifference { a: self, b: other }
    }

    /// Visits (potentially in parallel) the values representing the symmetric
    /// difference, i.e. the values that are in `self` or in `other` but not in both.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn par_symmetric_difference<'a>(
        &'a self,
        other: &'a Self,
    ) -> ParSymmetricDifference<'a, T, S, A> {
        ParSymmetricDifference { a: self, b: other }
    }

    /// Visits (potentially in parallel) the values representing the
    /// intersection, i.e. the values that are both in `self` and `other`.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn par_intersection<'a>(&'a self, other: &'a Self) -> ParIntersection<'a, T, S, A> {
        ParIntersection { a: self, b: other }
    }

    /// Returns `true` if `self` has no elements in common with `other`.
    /// This is equivalent to checking for an empty intersection.
    ///
    /// This method runs in a potentially parallel fashion.
    pub fn par_is_disjoint(&self, other: &Self) -> bool {
        self.into_par_iter().all(|x| !other.contains(x))
    }

    /// Returns `true` if the set is a subset of another,
    /// i.e. `other` contains at least all the values in `self`.
    ///
    /// This method runs in a potentially parallel fashion.
    pub fn par_is_subset(&self, other: &Self) -> bool {
        if self.len() <= other.len() {
            self.into_par_iter().all(|x| other.contains(x))
        } else {
            false
        }
    }

    /// Returns `true` if the set is a superset of another,
    /// i.e. `self` contains at least all the values in `other`.
    ///
    /// This method runs in a potentially parallel fashion.
    pub fn par_is_superset(&self, other: &Self) -> bool {
        other.par_is_subset(self)
    }

    /// Returns `true` if the set is equal to another,
    /// i.e. both sets contain the same values.
    ///
    /// This method runs in a potentially parallel fashion.
    pub fn par_eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.par_is_subset(other)
    }
}

impl<T, S, A> HashSet<T, S, A>
where
    T: Eq + Hash + Send,
    A: Allocator + Send,
{
    /// Consumes (potentially in parallel) all values in an arbitrary order,
    /// while preserving the set's allocated memory for reuse.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn par_drain(&mut self) -> ParDrain<'_, T, A> {
        ParDrain {
            inner: self.map.par_drain(),
        }
    }
}

impl<T: Send, S, A: Allocator + Send> IntoParallelIterator for HashSet<T, S, A> {
    type Item = T;
    type Iter = IntoParIter<T, A>;

    #[cfg_attr(feature = "inline-more", inline)]
    fn into_par_iter(self) -> Self::Iter {
        IntoParIter {
            inner: self.map.into_par_iter(),
        }
    }
}

impl<'a, T: Sync, S, A: Allocator> IntoParallelIterator for &'a HashSet<T, S, A> {
    type Item = &'a T;
    type Iter = ParIter<'a, T>;

    #[cfg_attr(feature = "inline-more", inline)]
    fn into_par_iter(self) -> Self::Iter {
        ParIter {
            inner: self.map.par_keys(),
        }
    }
}

/// Collect values from a parallel iterator into a hashset.
impl<T, S> FromParallelIterator<T> for HashSet<T, S, Global>
where
    T: Eq + Hash + Send,
    S: BuildHasher + Default,
{
    fn from_par_iter<P>(par_iter: P) -> Self
    where
        P: IntoParallelIterator<Item = T>,
    {
        let mut set = HashSet::default();
        set.par_extend(par_iter);
        set
    }
}

/// Extend a hash set with items from a parallel iterator.
impl<T, S> ParallelExtend<T> for HashSet<T, S, Global>
where
    T: Eq + Hash + Send,
    S: BuildHasher,
{
    fn par_extend<I>(&mut self, par_iter: I)
    where
        I: IntoParallelIterator<Item = T>,
    {
        extend(self, par_iter);
    }
}

/// Extend a hash set with copied items from a parallel iterator.
impl<'a, T, S> ParallelExtend<&'a T> for HashSet<T, S, Global>
where
    T: 'a + Copy + Eq + Hash + Sync,
    S: BuildHasher,
{
    fn par_extend<I>(&mut self, par_iter: I)
    where
        I: IntoParallelIterator<Item = &'a T>,
    {
        extend(self, par_iter);
    }
}

// This is equal to the normal `HashSet` -- no custom advantage.
fn extend<T, S, I, A>(set: &mut HashSet<T, S, A>, par_iter: I)
where
    T: Eq + Hash,
    S: BuildHasher,
    A: Allocator,
    I: IntoParallelIterator,
    HashSet<T, S, A>: Extend<I::Item>,
{
    let (list, len) = super::helpers::collect(par_iter);

    // Values may be already present or show multiple times in the iterator.
    // Reserve the entire length if the set is empty.
    // Otherwise reserve half the length (rounded up), so the set
    // will only resize twice in the worst case.
    let reserve = if set.is_empty() { len } else { len.div_ceil(2) };
    set.reserve(reserve);
    for vec in list {
        set.extend(vec);
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/hashbrown-0.17.1/unit/src/external_trait_impls/rayon/set/test_par_set_387.rs"));
