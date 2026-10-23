//! Rayon extensions for `HashMap`.

use super::raw::{RawIntoParIter, RawParDrain, RawParIter};
use crate::HashMap;
use crate::alloc::{Allocator, Global};
use core::fmt;
use core::hash::{BuildHasher, Hash};
use core::marker::PhantomData;
use rayon::iter::plumbing::UnindexedConsumer;
use rayon::iter::{FromParallelIterator, IntoParallelIterator, ParallelExtend, ParallelIterator};

/// Parallel iterator over shared references to entries in a map.
///
/// This iterator is created by the [`par_iter`] method on [`HashMap`]
/// (provided by the [`IntoParallelRefIterator`] trait).
/// See its documentation for more.
///
/// [`par_iter`]: rayon::iter::IntoParallelRefIterator::par_iter
/// [`IntoParallelRefIterator`]: rayon::iter::IntoParallelRefIterator
pub struct ParIter<'a, K, V> {
    inner: RawParIter<(K, V)>,
    marker: PhantomData<(&'a K, &'a V)>,
}

impl<'a, K: Sync, V: Sync> ParallelIterator for ParIter<'a, K, V> {
    type Item = (&'a K, &'a V);

    #[cfg_attr(feature = "inline-more", inline)]
    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        self.inner
            .map(|x| unsafe {
                let r = x.as_ref();
                (&r.0, &r.1)
            })
            .drive_unindexed(consumer)
    }
}

impl<K, V> Clone for ParIter<'_, K, V> {
    #[cfg_attr(feature = "inline-more", inline)]
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            marker: PhantomData,
        }
    }
}

impl<K: fmt::Debug + Eq + Hash, V: fmt::Debug> fmt::Debug for ParIter<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let iter = unsafe { self.inner.iter() }.map(|x| unsafe {
            let r = x.as_ref();
            (&r.0, &r.1)
        });
        f.debug_list().entries(iter).finish()
    }
}

/// Parallel iterator over shared references to keys in a map.
///
/// This iterator is created by the [`par_keys`] method on [`HashMap`].
/// See its documentation for more.
///
/// [`par_keys`]: HashMap::par_keys
pub struct ParKeys<'a, K, V> {
    inner: RawParIter<(K, V)>,
    marker: PhantomData<(&'a K, &'a V)>,
}

impl<'a, K: Sync, V: Sync> ParallelIterator for ParKeys<'a, K, V> {
    type Item = &'a K;

    #[cfg_attr(feature = "inline-more", inline)]
    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        self.inner
            .map(|x| unsafe { &x.as_ref().0 })
            .drive_unindexed(consumer)
    }
}

impl<K, V> Clone for ParKeys<'_, K, V> {
    #[cfg_attr(feature = "inline-more", inline)]
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            marker: PhantomData,
        }
    }
}

impl<K: fmt::Debug + Eq + Hash, V> fmt::Debug for ParKeys<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let iter = unsafe { self.inner.iter() }.map(|x| unsafe { &x.as_ref().0 });
        f.debug_list().entries(iter).finish()
    }
}

/// Parallel iterator over shared references to values in a map.
///
/// This iterator is created by the [`par_values`] method on [`HashMap`].
/// See its documentation for more.
///
/// [`par_values`]: HashMap::par_values
pub struct ParValues<'a, K, V> {
    inner: RawParIter<(K, V)>,
    marker: PhantomData<(&'a K, &'a V)>,
}

impl<'a, K: Sync, V: Sync> ParallelIterator for ParValues<'a, K, V> {
    type Item = &'a V;

    #[cfg_attr(feature = "inline-more", inline)]
    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        self.inner
            .map(|x| unsafe { &x.as_ref().1 })
            .drive_unindexed(consumer)
    }
}

impl<K, V> Clone for ParValues<'_, K, V> {
    #[cfg_attr(feature = "inline-more", inline)]
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            marker: PhantomData,
        }
    }
}

impl<K: Eq + Hash, V: fmt::Debug> fmt::Debug for ParValues<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let iter = unsafe { self.inner.iter() }.map(|x| unsafe { &x.as_ref().1 });
        f.debug_list().entries(iter).finish()
    }
}

/// Parallel iterator over mutable references to entries in a map.
///
/// This iterator is created by the [`par_iter_mut`] method on [`HashMap`]
/// (provided by the [`IntoParallelRefMutIterator`] trait).
/// See its documentation for more.
///
/// [`par_iter_mut`]: rayon::iter::IntoParallelRefMutIterator::par_iter_mut
/// [`IntoParallelRefMutIterator`]: rayon::iter::IntoParallelRefMutIterator
pub struct ParIterMut<'a, K, V> {
    inner: RawParIter<(K, V)>,
    marker: PhantomData<(&'a K, &'a mut V)>,
}

impl<'a, K: Sync, V: Send> ParallelIterator for ParIterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);

    #[cfg_attr(feature = "inline-more", inline)]
    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        self.inner
            .map(|x| unsafe {
                let r = x.as_mut();
                (&r.0, &mut r.1)
            })
            .drive_unindexed(consumer)
    }
}

impl<K: fmt::Debug + Eq + Hash, V: fmt::Debug> fmt::Debug for ParIterMut<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        ParIter {
            inner: self.inner.clone(),
            marker: PhantomData,
        }
        .fmt(f)
    }
}

/// Parallel iterator over mutable references to values in a map.
///
/// This iterator is created by the [`par_values_mut`] method on [`HashMap`].
/// See its documentation for more.
///
/// [`par_values_mut`]: HashMap::par_values_mut
pub struct ParValuesMut<'a, K, V> {
    inner: RawParIter<(K, V)>,
    marker: PhantomData<(&'a K, &'a mut V)>,
}

impl<'a, K: Sync, V: Send> ParallelIterator for ParValuesMut<'a, K, V> {
    type Item = &'a mut V;

    #[cfg_attr(feature = "inline-more", inline)]
    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        self.inner
            .map(|x| unsafe { &mut x.as_mut().1 })
            .drive_unindexed(consumer)
    }
}

impl<K: Eq + Hash, V: fmt::Debug> fmt::Debug for ParValuesMut<'_, K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        ParValues {
            inner: self.inner.clone(),
            marker: PhantomData,
        }
        .fmt(f)
    }
}

/// Parallel iterator over entries of a consumed map.
///
/// This iterator is created by the [`into_par_iter`] method on [`HashMap`]
/// (provided by the [`IntoParallelIterator`] trait).
/// See its documentation for more.
///
/// [`into_par_iter`]: HashMap::into_par_iter
pub struct IntoParIter<K, V, A: Allocator = Global> {
    inner: RawIntoParIter<(K, V), A>,
}

impl<K: Send, V: Send, A: Allocator + Send> ParallelIterator for IntoParIter<K, V, A> {
    type Item = (K, V);

    #[cfg_attr(feature = "inline-more", inline)]
    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        self.inner.drive_unindexed(consumer)
    }
}

impl<K: fmt::Debug + Eq + Hash, V: fmt::Debug, A: Allocator> fmt::Debug for IntoParIter<K, V, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        ParIter {
            inner: unsafe { self.inner.par_iter() },
            marker: PhantomData,
        }
        .fmt(f)
    }
}

/// Parallel draining iterator over entries of a map.
///
/// This iterator is created by the [`par_drain`] method on [`HashMap`].
/// See its documentation for more.
///
/// [`par_drain`]: HashMap::par_drain
pub struct ParDrain<'a, K, V, A: Allocator = Global> {
    inner: RawParDrain<'a, (K, V), A>,
}

impl<K: Send, V: Send, A: Allocator + Sync> ParallelIterator for ParDrain<'_, K, V, A> {
    type Item = (K, V);

    #[cfg_attr(feature = "inline-more", inline)]
    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        self.inner.drive_unindexed(consumer)
    }
}

impl<K: fmt::Debug + Eq + Hash, V: fmt::Debug, A: Allocator> fmt::Debug for ParDrain<'_, K, V, A> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        ParIter {
            inner: unsafe { self.inner.par_iter() },
            marker: PhantomData,
        }
        .fmt(f)
    }
}

impl<K: Sync, V: Sync, S, A: Allocator> HashMap<K, V, S, A> {
    /// Visits (potentially in parallel) immutably borrowed keys in an arbitrary order.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn par_keys(&self) -> ParKeys<'_, K, V> {
        ParKeys {
            inner: unsafe { self.table.par_iter() },
            marker: PhantomData,
        }
    }

    /// Visits (potentially in parallel) immutably borrowed values in an arbitrary order.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn par_values(&self) -> ParValues<'_, K, V> {
        ParValues {
            inner: unsafe { self.table.par_iter() },
            marker: PhantomData,
        }
    }
}

impl<K: Send, V: Send, S, A: Allocator> HashMap<K, V, S, A> {
    /// Visits (potentially in parallel) mutably borrowed values in an arbitrary order.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn par_values_mut(&mut self) -> ParValuesMut<'_, K, V> {
        ParValuesMut {
            inner: unsafe { self.table.par_iter() },
            marker: PhantomData,
        }
    }

    /// Consumes (potentially in parallel) all values in an arbitrary order,
    /// while preserving the map's allocated memory for reuse.
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn par_drain(&mut self) -> ParDrain<'_, K, V, A> {
        ParDrain {
            inner: self.table.par_drain(),
        }
    }
}

impl<K, V, S, A> HashMap<K, V, S, A>
where
    K: Eq + Hash + Sync,
    V: PartialEq + Sync,
    S: BuildHasher + Sync,
    A: Allocator + Sync,
{
    /// Returns `true` if the map is equal to another,
    /// i.e. both maps contain the same keys mapped to the same values.
    ///
    /// This method runs in a potentially parallel fashion.
    pub fn par_eq(&self, other: &Self) -> bool {
        self.len() == other.len()
            && self
                .into_par_iter()
                .all(|(key, value)| other.get(key).is_some_and(|v| *value == *v))
    }
}

impl<K: Send, V: Send, S, A: Allocator + Send> IntoParallelIterator for HashMap<K, V, S, A> {
    type Item = (K, V);
    type Iter = IntoParIter<K, V, A>;

    #[cfg_attr(feature = "inline-more", inline)]
    fn into_par_iter(self) -> Self::Iter {
        IntoParIter {
            inner: self.table.into_par_iter(),
        }
    }
}

impl<'a, K: Sync, V: Sync, S, A: Allocator> IntoParallelIterator for &'a HashMap<K, V, S, A> {
    type Item = (&'a K, &'a V);
    type Iter = ParIter<'a, K, V>;

    #[cfg_attr(feature = "inline-more", inline)]
    fn into_par_iter(self) -> Self::Iter {
        ParIter {
            inner: unsafe { self.table.par_iter() },
            marker: PhantomData,
        }
    }
}

impl<'a, K: Sync, V: Send, S, A: Allocator> IntoParallelIterator for &'a mut HashMap<K, V, S, A> {
    type Item = (&'a K, &'a mut V);
    type Iter = ParIterMut<'a, K, V>;

    #[cfg_attr(feature = "inline-more", inline)]
    fn into_par_iter(self) -> Self::Iter {
        ParIterMut {
            inner: unsafe { self.table.par_iter() },
            marker: PhantomData,
        }
    }
}

/// Collect (key, value) pairs from a parallel iterator into a
/// hashmap. If multiple pairs correspond to the same key, then the
/// ones produced earlier in the parallel iterator will be
/// overwritten, just as with a sequential iterator.
impl<K, V, S> FromParallelIterator<(K, V)> for HashMap<K, V, S, Global>
where
    K: Eq + Hash + Send,
    V: Send,
    S: BuildHasher + Default,
{
    fn from_par_iter<P>(par_iter: P) -> Self
    where
        P: IntoParallelIterator<Item = (K, V)>,
    {
        let mut map = HashMap::default();
        map.par_extend(par_iter);
        map
    }
}

/// Extend a hash map with items from a parallel iterator.
impl<K, V, S, A> ParallelExtend<(K, V)> for HashMap<K, V, S, A>
where
    K: Eq + Hash + Send,
    V: Send,
    S: BuildHasher,
    A: Allocator,
{
    fn par_extend<I>(&mut self, par_iter: I)
    where
        I: IntoParallelIterator<Item = (K, V)>,
    {
        extend(self, par_iter);
    }
}

/// Extend a hash map with copied items from a parallel iterator.
impl<'a, K, V, S, A> ParallelExtend<(&'a K, &'a V)> for HashMap<K, V, S, A>
where
    K: Copy + Eq + Hash + Sync,
    V: Copy + Sync,
    S: BuildHasher,
    A: Allocator,
{
    fn par_extend<I>(&mut self, par_iter: I)
    where
        I: IntoParallelIterator<Item = (&'a K, &'a V)>,
    {
        extend(self, par_iter);
    }
}

// This is equal to the normal `HashMap` -- no custom advantage.
fn extend<K, V, S, A, I>(map: &mut HashMap<K, V, S, A>, par_iter: I)
where
    K: Eq + Hash,
    S: BuildHasher,
    I: IntoParallelIterator,
    A: Allocator,
    HashMap<K, V, S, A>: Extend<I::Item>,
{
    let (list, len) = super::helpers::collect(par_iter);

    // Keys may be already present or show multiple times in the iterator.
    // Reserve the entire length if the map is empty.
    // Otherwise reserve half the length (rounded up), so the map
    // will only resize twice in the worst case.
    let reserve = if map.is_empty() { len } else { len.div_ceil(2) };
    map.reserve(reserve);
    for vec in list {
        map.extend(vec);
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/hash-table/unit/src/external_trait_impls/rayon/map/test_par_map_457.rs"));
