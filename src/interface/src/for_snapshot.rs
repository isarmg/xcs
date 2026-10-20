//! ZXS snapshot lists combine the Leptos-derived row ownership and Tachys keyed
//! DOM reconciler with per-key reactive payloads. Server snapshots can change a
//! transfer's status without replacing its DOM, focus, or expanded details.
use crate::IntoView;
use reactive_graph::{
    owner::Owner,
    signal::{ArcRwSignal, ReadSignal},
    traits::{Set, WithUntracked},
};
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    sync::{Arc, Mutex},
};
use tachys::{reactive_graph::OwnedView, view::keyed::keyed};
use view_macros::component;

/// Retains keyed row DOM while publishing changed snapshot values to each row.
///
/// Unlike [`crate::control_flow::For`], an existing key receives new data through
/// its read-only signal. Read fields inside reactive closures in `children`.
/// Equal payloads do not notify rows. Removing a key releases its row owner;
/// adding that key again creates a fresh row. Keys must be unique per snapshot.
/// A duplicate key panics before the cache or any row payload is changed.
#[component]
pub fn ForSnapshot<IF, I, T, EF, N, KF, K>(
    /// Current immutable snapshot of the visible rows.
    each: IF,
    /// Stable identity, independent of mutable status or display text.
    key: KF,
    /// Builds a row once and receives its current payload as a read-only signal.
    children: EF,
) -> impl IntoView
where
    IF: Fn() -> I + Send + 'static,
    I: IntoIterator<Item = T> + Send + 'static,
    T: PartialEq + Send + Sync + 'static,
    EF: Fn(ReadSignal<T>) -> N + Send + Clone + 'static,
    N: IntoView + 'static,
    KF: Fn(&T) -> K + Send + 'static,
    K: Clone + Eq + Hash + Send + 'static,
{
    let parent = Owner::current().expect("no reactive owner");
    let rows = Arc::new(SnapshotRows::default());
    let children = move |_, (_, snapshot): (K, ArcRwSignal<T>)| {
        let owner = parent.with(Owner::new);
        let view = owner.with(|| children(snapshot.read_only().into()));
        (drop, OwnedView::new_with_owner(view, owner))
    };
    move || {
        let items = each()
            .into_iter()
            .map(|item| (key(&item), item))
            .collect::<Vec<_>>();
        let items = rows.reconcile(items);
        keyed(items, |(key, _)| key.clone(), children.clone())
    }
}

struct SnapshotRows<K, T> {
    rows: Mutex<HashMap<K, ArcRwSignal<T>>>,
}
impl<K, T> Default for SnapshotRows<K, T> {
    fn default() -> Self {
        Self {
            rows: Mutex::new(HashMap::new()),
        }
    }
}
struct PreparedSnapshot<K, T> {
    items: Vec<(K, ArcRwSignal<T>)>,
    updates: Vec<(ArcRwSignal<T>, T)>,
    removed: HashMap<K, ArcRwSignal<T>>,
}
impl<K: Clone + Eq + Hash, T: PartialEq + Send + Sync + 'static> SnapshotRows<K, T> {
    fn reconcile(&self, items: Vec<(K, T)>) -> Vec<(K, ArcRwSignal<T>)> {
        // Reject duplicate identities before taking the cache lock. Failure
        // cannot partly update visible rows or poison the registry's mutex.
        let mut seen = HashSet::with_capacity(items.len());
        assert!(
            items.iter().all(|(key, _)| seen.insert(key.clone())),
            "ForSnapshot requires unique row keys"
        );
        let mut rows = self.rows.lock().unwrap();
        let mut previous = std::mem::take(&mut *rows);
        let mut retained = Vec::with_capacity(items.len());
        let mut updates = Vec::new();
        rows.reserve(items.len());
        for (key, value) in items {
            let signal = match previous.remove(&key) {
                Some(signal) => {
                    updates.push((signal.clone(), value));
                    signal
                }
                None => ArcRwSignal::new(value),
            };
            rows.insert(key.clone(), signal.clone());
            retained.push((key, signal));
        }
        drop(rows);
        // Equality, notification, and removed payload destructors run outside
        // the registry lock; user code may execute during any of these steps.
        PreparedSnapshot {
            items: retained,
            updates,
            removed: previous,
        }
        .publish()
    }
}
impl<K, T: PartialEq + Send + Sync + 'static> PreparedSnapshot<K, T> {
    fn publish(self) -> Vec<(K, ArcRwSignal<T>)> {
        for (signal, value) in self.updates {
            if signal.with_untracked(|current| current != &value) {
                signal.set(value);
            }
        }
        drop(self.removed);
        self.items
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reactive_graph::{computed::ArcMemo, traits::GetUntracked};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn snapshot_updates_preserve_identity_across_changes_and_reordering() {
        let rows = SnapshotRows::default();
        let first = rows.reconcile(vec![("a", "uploading"), ("b", "waiting")]);
        let next = rows.reconcile(vec![("b", "saving"), ("a", "published")]);
        assert_eq!(next[0].0, "b");
        assert_eq!(next[0].1, first[1].1);
        assert_eq!(next[1].1, first[0].1);
        assert_eq!(first[0].1.get_untracked(), "published");
        assert_eq!(first[1].1.get_untracked(), "saving");
    }

    #[test]
    fn equal_snapshots_do_not_invalidate_unchanged_row_consumers() {
        let rows = SnapshotRows::default();
        let initial = rows.reconcile(vec![(1, "uploading")]);
        let runs = Arc::new(AtomicUsize::new(0));
        let observed_runs = runs.clone();
        let signal = initial[0].1.clone();
        let memo = ArcMemo::new(move |_| {
            observed_runs.fetch_add(1, Ordering::Relaxed);
            use reactive_graph::traits::Get;
            signal.get()
        });
        assert_eq!(memo.get_untracked(), "uploading");
        for _ in 0..1_000 {
            rows.reconcile(vec![(1, "uploading")]);
            assert_eq!(memo.get_untracked(), "uploading");
        }
        assert_eq!(runs.load(Ordering::Relaxed), 1);
        rows.reconcile(vec![(1, "published")]);
        assert_eq!(memo.get_untracked(), "published");
        assert_eq!(runs.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn removed_keys_leave_the_cache_and_reinsertion_gets_fresh_state() {
        let rows = SnapshotRows::default();
        let initial = rows.reconcile(vec![(1, "old")]);
        assert!(rows.reconcile(Vec::new()).is_empty());
        assert!(rows.rows.lock().unwrap().is_empty());
        let inserted = rows.reconcile(vec![(1, "new")]);
        assert_ne!(initial[0].1, inserted[0].1);
        assert_eq!(initial[0].1.get_untracked(), "old");
        assert_eq!(inserted[0].1.get_untracked(), "new");
    }

    #[test]
    fn duplicate_snapshot_cannot_partially_publish_or_poison_the_registry() {
        let rows = SnapshotRows::default();
        let first = rows.reconcile(vec![(1, "uploading")]);
        assert!(
            std::panic::catch_unwind(|| {
                rows.reconcile(vec![(1, "published"), (1, "rejected")]);
            })
            .is_err()
        );
        assert_eq!(first[0].1.get_untracked(), "uploading");
        let next = rows.reconcile(vec![(1, "saving")]);
        assert_eq!(first[0].1, next[0].1);
        assert_eq!(first[0].1.get_untracked(), "saving");
    }
}
