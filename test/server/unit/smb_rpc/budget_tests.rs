use super::*;

fn limits(account: u64, total: u64) -> SpoolLimits {
    SpoolLimits {
        bytes_per_account: account,
        bytes_total: total,
        recovery_reserve_bytes: 1,
    }
}
fn reserve(
    budget: &Arc<Budget>,
    principal: &str,
    name: &str,
    ceiling: u64,
) -> Result<Lease, ApiError> {
    budget.reserve_available(
        principal,
        &budget.root.join(name),
        ceiling,
        ceiling,
        u64::MAX,
    )
}
fn total(budget: &Budget) -> u64 {
    budget
        .ledger
        .lock()
        .unwrap()
        .files
        .values()
        .map(|entry| entry.bytes)
        .sum()
}
#[test]
fn retained_descriptor_ceiling_survives_disconnect_and_restart() {
    let root = tempfile::tempdir().unwrap();
    let limit = OVERHEAD + 100;
    let budget = Budget::recover(root.path(), limits(limit, 2 * limit)).unwrap();
    let lease = reserve(&budget, "alice", "copy", 100).unwrap();
    fs::write(&lease.source, b"one").unwrap();
    // A flushed or disconnected descriptor can still expand later. Charge the
    // full ceiling, not the current three bytes, even after its server drops.
    drop(lease);
    assert_eq!(total(&budget), limit);
    assert!(reserve(&budget, "alice", "other", 0).is_err());
    drop(budget);
    let budget = Budget::recover(root.path(), limits(limit, 2 * limit)).unwrap();
    assert_eq!(total(&budget), limit);
    assert!(reserve(&budget, "alice", "other", 0).is_err());
    assert!(reserve(&budget, "bob", "other", 0).is_ok());
    assert_eq!(fs::read(root.path().join("copy")).unwrap(), b"one");
}
#[test]
fn successful_cleanup_releases_capacity_but_failed_cleanup_does_not() {
    let root = tempfile::tempdir().unwrap();
    let limit = OVERHEAD + 100;
    let budget = Budget::recover(root.path(), limits(limit, limit)).unwrap();
    let lease = reserve(&budget, "alice", "copy", 100).unwrap();
    fs::write(&lease.source, b"one").unwrap();
    fs::write(lease.source.with_extension("work.json"), b"evidence").unwrap();
    fs::remove_file(&lease.source).unwrap();
    drop(lease);
    assert_eq!(total(&budget), limit);
    assert!(root.path().join("copy.budget.json").exists());
    // A separate clean lease demonstrates the normal Close path.
    let clean = tempfile::tempdir().unwrap();
    let budget = Budget::recover(clean.path(), limits(limit, limit)).unwrap();
    let lease = reserve(&budget, "alice", "copy", 100).unwrap();
    fs::write(&lease.source, b"one").unwrap();
    fs::remove_file(&lease.source).unwrap();
    drop(lease);
    assert_eq!(total(&budget), 0);
    assert!(!clean.path().join("copy.budget.json").exists());
}
#[test]
fn global_account_and_recovery_admission_precede_payload_expansion() {
    let root = tempfile::tempdir().unwrap();
    let limit = OVERHEAD + 100;
    let budget = Budget::recover(root.path(), limits(limit, 2 * limit)).unwrap();
    assert!(
        budget
            .reserve_available("alice", &root.path().join("a"), 100, 100, limit)
            .is_err()
    );
    assert!(!root.path().join("a.budget.json").exists());
    let _a = reserve(&budget, "alice", "a", 100).unwrap();
    assert!(reserve(&budget, "alice", "b", 1).is_err());
    let _b = reserve(&budget, "bob", "b", 100).unwrap();
    assert!(reserve(&budget, "charlie", "c", 0).is_err());
}
#[test]
fn orphan_bytes_charge_every_account_without_deletion() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("legacy"), b"unknown").unwrap();
    let budget = Budget::recover(root.path(), limits(OVERHEAD + 100, 3 * OVERHEAD)).unwrap();
    assert!(reserve(&budget, "alice", "new", 0).is_err());
    assert!(reserve(&budget, "bob", "new", 0).is_err());
    assert_eq!(fs::read(root.path().join("legacy")).unwrap(), b"unknown");
}
#[test]
fn ceiling_checks_happen_before_write_and_metadata_corruption_fails_closed() {
    let root = tempfile::tempdir().unwrap();
    let budget = Budget::recover(root.path(), limits(OVERHEAD + 100, OVERHEAD + 100)).unwrap();
    let lease = reserve(&budget, "alice", "copy", 100).unwrap();
    assert!(lease.before_write_available(101, 0, u64::MAX).is_err());
    assert!(lease.before_write_available(100, 100, 100).is_err());
    assert!(lease.before_write_available(100, 100, 101).is_ok());
    fs::write(&lease.source, b"one").unwrap();
    drop(lease);
    drop(budget);
    fs::write(root.path().join("copy.budget.json"), b"invalid").unwrap();
    assert!(Budget::recover(root.path(), limits(OVERHEAD + 100, OVERHEAD + 100)).is_err());
    assert_eq!(fs::read(root.path().join("copy")).unwrap(), b"one");
}

#[test]
fn concurrent_admission_cannot_overbook_a_single_account() {
    let root = tempfile::tempdir().unwrap();
    let one = OVERHEAD + 1;
    let budget = Budget::recover(root.path(), limits(2 * one, 8 * one)).unwrap();
    let barrier = Arc::new(std::sync::Barrier::new(8));
    let workers = (0..8)
        .map(|index| {
            let budget = budget.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let admitted = reserve(&budget, "alice", &format!("copy{index}"), 1);
                barrier.wait();
                let success = admitted.is_ok();
                barrier.wait();
                success
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        workers
            .into_iter()
            .filter_map(|worker| worker.join().ok())
            .filter(|accepted| *accepted)
            .count(),
        2
    );
}

#[test]
fn tiny_open_reserves_current_extent_not_static_maximum_and_growth_is_durable() {
    let root = tempfile::tempdir().unwrap();
    let limit = 2 * OVERHEAD + 20;
    let budget = Budget::recover(root.path(), limits(limit, limit)).unwrap();
    let lease = budget
        .reserve_available(
            "alice",
            &root.path().join("tiny"),
            3,
            1024 * 1024 * 1024,
            u64::MAX,
        )
        .unwrap();
    fs::write(&lease.source, b"old").unwrap();
    let _other = budget
        .reserve_available(
            "alice",
            &root.path().join("other"),
            0,
            1024 * 1024 * 1024,
            u64::MAX,
        )
        .unwrap();
    assert_eq!(total(&budget), 2 * OVERHEAD + 3);
    lease.before_write_available(20, 17, u64::MAX).unwrap();
    assert_eq!(total(&budget), limit);
    assert!(lease.before_write_available(21, 1, u64::MAX).is_err());
    assert_eq!(
        fs::read(&lease.source).unwrap(),
        b"old",
        "admission does not write payload"
    );
    // A later truncate/overwrite cannot release an earlier admitted extent.
    lease.before_write_available(1, 1, u64::MAX).unwrap();
    drop(lease);
    drop(_other);
    drop(budget);
    let budget = Budget::recover(root.path(), limits(limit, limit)).unwrap();
    assert_eq!(total(&budget), OVERHEAD + 20);
    assert_eq!(fs::read(root.path().join("tiny")).unwrap(), b"old");
}

#[test]
fn failed_growth_record_keeps_data_and_blocks_new_admission() {
    let root = tempfile::tempdir().unwrap();
    let budget = Budget::recover(root.path(), limits(4 * OVERHEAD, 4 * OVERHEAD)).unwrap();
    let lease = budget
        .reserve_available("alice", &root.path().join("copy"), 3, 100, u64::MAX)
        .unwrap();
    fs::write(&lease.source, b"old").unwrap();
    // Force atomic record replacement to fail without relying on root-sensitive
    // permission tests or actually exhausting the shared filesystem.
    fs::remove_file(record_path(&lease.source)).unwrap();
    fs::create_dir(record_path(&lease.source)).unwrap();
    assert!(lease.before_write_available(20, 17, u64::MAX).is_err());
    assert_eq!(total(&budget), OVERHEAD + 20);
    assert!(reserve(&budget, "bob", "other", 0).is_err());
    assert_eq!(fs::read(&lease.source).unwrap(), b"old");
    drop(lease);
    drop(budget);
    assert!(Budget::recover(root.path(), limits(4 * OVERHEAD, 4 * OVERHEAD)).is_err());
    assert_eq!(fs::read(root.path().join("copy")).unwrap(), b"old");
}
