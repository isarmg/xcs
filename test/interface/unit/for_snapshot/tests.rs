use super::*;
use reactive_state::{computed::ArcMemo, traits::GetUntracked};
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
        use reactive_state::traits::Get;
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
