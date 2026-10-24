//! Types that hold the set of sources or subscribers affiliated with a reactive node.
//!
//! These use insertion-ordered hash sets so subscriptions are deduplicated without
//! scanning the existing entries, while notifications retain subscription order.

use super::{AnySource, AnySubscriber, Source};
use indexmap::IndexSet;
use rustc_hash::FxHasher;
use std::{hash::BuildHasherDefault, mem};

type FxIndexSet<T> = IndexSet<T, BuildHasherDefault<FxHasher>>;

#[derive(Default, Clone, Debug)]
pub struct SourceSet(FxIndexSet<AnySource>);

impl SourceSet {
    pub fn new() -> Self {
        Self(Default::default())
    }

    pub fn insert(&mut self, source: AnySource) {
        self.0.insert(source);
    }

    pub fn remove(&mut self, source: &AnySource) {
        self.0.shift_remove(source);
    }

    pub fn take(&mut self) -> FxIndexSet<AnySource> {
        mem::take(&mut self.0)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn clear_sources(&mut self, subscriber: &AnySubscriber) {
        for source in self.take() {
            source.remove_subscriber(subscriber);
        }
    }
}

impl IntoIterator for SourceSet {
    type Item = AnySource;
    type IntoIter = <FxIndexSet<AnySource> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a SourceSet {
    type Item = &'a AnySource;
    type IntoIter = <&'a FxIndexSet<AnySource> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
#[derive(Debug, Default, Clone)]
pub struct SubscriberSet(FxIndexSet<AnySubscriber>);

impl SubscriberSet {
    pub fn new() -> Self {
        Self(FxIndexSet::with_capacity_and_hasher(2, Default::default()))
    }

    pub fn subscribe(&mut self, subscriber: AnySubscriber) {
        self.0.insert(subscriber);
    }

    /// Prunes dead subscribers and snapshots the remaining notification order.
    /// Callers must release the source lock before notifying this snapshot.
    pub(crate) fn live_snapshot(&mut self) -> Self {
        // Do not upgrade under the source lock: dropping a temporary last strong
        // reference could run subscriber cleanup that re-enters this source.
        self.0.retain(|subscriber| subscriber.1.strong_count() != 0);
        self.clone()
    }

    pub fn unsubscribe(&mut self, subscriber: &AnySubscriber) {
        // note: do not use `.swap_remove()` here.
        // using `.remove()` is slower because it shifts other items
        // but it maintains the order of the subscribers, which is important
        // to correctness when you're using this to drive something like a UI,
        // which can have nested effects, where the inner one assumes the outer
        // has already run (for example, an outer effect that checks .is_some(),
        // and an inner effect that unwraps)
        self.0.shift_remove(subscriber);
    }

    pub fn take(&mut self) -> FxIndexSet<AnySubscriber> {
        mem::take(&mut self.0)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl IntoIterator for SubscriberSet {
    type Item = AnySubscriber;
    type IntoIter = <FxIndexSet<AnySubscriber> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a SubscriberSet {
    type Item = &'a AnySubscriber;
    type IntoIter = <&'a FxIndexSet<AnySubscriber> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
