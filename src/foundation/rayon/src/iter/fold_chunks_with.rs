use std::fmt::{self, Debug};

use super::chunks::ChunkProducer;
use super::*;

/// `FoldChunksWith` is an iterator that groups elements of an underlying iterator and applies a
/// function over them, producing a single value for each group.
///
/// This struct is created by the [`fold_chunks_with()`] method on [`IndexedParallelIterator`]
///
/// [`fold_chunks_with()`]: IndexedParallelIterator::fold_chunks()
#[must_use = "iterator adaptors are lazy and do nothing unless consumed"]
#[derive(Clone)]
pub struct FoldChunksWith<I, U, F> {
    base: I,
    chunk_size: usize,
    item: U,
    fold_op: F,
}

impl<I: Debug, U: Debug, F> Debug for FoldChunksWith<I, U, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Fold")
            .field("base", &self.base)
            .field("chunk_size", &self.chunk_size)
            .field("item", &self.item)
            .finish()
    }
}

impl<I, U, F> FoldChunksWith<I, U, F> {
    /// Creates a new `FoldChunksWith` iterator
    pub(super) fn new(base: I, chunk_size: usize, item: U, fold_op: F) -> Self {
        FoldChunksWith {
            base,
            chunk_size,
            item,
            fold_op,
        }
    }
}

impl<I, U, F> ParallelIterator for FoldChunksWith<I, U, F>
where
    I: IndexedParallelIterator,
    U: Send + Clone,
    F: Fn(U, I::Item) -> U + Send + Sync,
{
    type Item = U;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: Consumer<U>,
    {
        bridge(self, consumer)
    }

    fn opt_len(&self) -> Option<usize> {
        Some(self.len())
    }
}

impl<I, U, F> IndexedParallelIterator for FoldChunksWith<I, U, F>
where
    I: IndexedParallelIterator,
    U: Send + Clone,
    F: Fn(U, I::Item) -> U + Send + Sync,
{
    fn len(&self) -> usize {
        self.base.len().div_ceil(self.chunk_size)
    }

    fn drive<C>(self, consumer: C) -> C::Result
    where
        C: Consumer<Self::Item>,
    {
        bridge(self, consumer)
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        let len = self.base.len();
        return self.base.with_producer(Callback {
            chunk_size: self.chunk_size,
            len,
            item: self.item,
            fold_op: self.fold_op,
            callback,
        });

        struct Callback<CB, T, F> {
            chunk_size: usize,
            len: usize,
            item: T,
            fold_op: F,
            callback: CB,
        }

        impl<T, U, F, CB> ProducerCallback<T> for Callback<CB, U, F>
        where
            CB: ProducerCallback<U>,
            U: Send + Clone,
            F: Fn(U, T) -> U + Send + Sync,
        {
            type Output = CB::Output;

            fn callback<P>(self, base: P) -> CB::Output
            where
                P: Producer<Item = T>,
            {
                let item = self.item;
                let fold_op = &self.fold_op;
                let fold_iter = move |iter: P::IntoIter| iter.fold(item.clone(), fold_op);
                let producer = ChunkProducer::new(self.chunk_size, self.len, base, fold_iter);
                self.callback.callback(producer)
            }
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/rayon/unit/src/iter/fold_chunks_with/test_123.rs"));
