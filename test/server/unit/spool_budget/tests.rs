use super::*;
use crate::uploads::{self, StagedFile};

fn limits(account: u64, total: u64) -> SpoolLimits {
    SpoolLimits {
        bytes_per_account: account,
        bytes_total: total,
        recovery_reserve_bytes: 1,
    }
}
fn retained(budget: &Arc<SpoolBudget>, principal: &str, name: &str, declared: u64, bytes: &[u8]) {
    let reservation = budget
        .reserve_with_available(principal, name, declared, u64::MAX)
        .unwrap();
    let staged = StagedFile::open_budgeted(budget.root.join(name), false, reservation).unwrap();
    staged.write(bytes).unwrap();
    staged.sync().unwrap();
}
fn total(budget: &SpoolBudget) -> u64 {
    budget
        .ledger
        .lock()
        .unwrap()
        .files
        .values()
        .map(|entry| entry.bytes)
        .sum()
}
fn record(name: &str, principal: &str) -> UploadRecord {
    UploadRecord {
        workspace_id: "ws".into(),
        item_id: "item".into(),
        principal: principal.into(),
        path: "file".into(),
        generation: 1,
        logical_size: 20,
        content_hash: Some("0".repeat(64)),
        persisted_bytes: 3,
        persisted_hash: blake3::hash(b"abc").to_hex().to_string(),
        state: "complete".into(),
        data_file: name.into(),
    }
}
#[test]
fn reservation_settles_to_retained_bytes_not_success_or_declared_length() {
    let root = tempfile::tempdir().unwrap();
    let budget = SpoolBudget::recover(
        root.path(),
        limits(2 * METADATA_ALLOWANCE + 13, 2 * METADATA_ALLOWANCE + 13),
    )
    .unwrap();
    retained(&budget, "alice", "a.data", 10, b"abc");
    assert_eq!(total(&budget), METADATA_ALLOWANCE + 3);
    let second = budget
        .reserve_with_available("alice", "b.data", 10, u64::MAX)
        .unwrap();
    assert_eq!(total(&budget), 2 * METADATA_ALLOWANCE + 13);
    assert!(
        budget
            .reserve_with_available("bob", "c.data", 0, u64::MAX)
            .is_err()
    );
    drop(second);
    assert_eq!(total(&budget), METADATA_ALLOWANCE + 3);
    // Same physical file has one charge, even at its full quota.
    let retry = budget
        .reserve_with_available("alice", "a.data", 10, u64::MAX)
        .unwrap();
    assert_eq!(total(&budget), METADATA_ALLOWANCE + 10);
    drop(retry);
    assert_eq!(total(&budget), METADATA_ALLOWANCE + 3);
}
#[test]
fn concurrent_reservations_cannot_oversubscribe_account_or_pool() {
    let root = tempfile::tempdir().unwrap();
    let charge = METADATA_ALLOWANCE + 1;
    let budget = SpoolBudget::recover(root.path(), limits(2 * charge, 3 * charge)).unwrap();
    let barrier = Arc::new(std::sync::Barrier::new(9));
    let mut threads = Vec::new();
    for index in 0..8 {
        let budget = budget.clone();
        let barrier = barrier.clone();
        threads.push(std::thread::spawn(move || {
            let reservation =
                budget.reserve_with_available("alice", &format!("{index}.data"), 1, u64::MAX);
            let accepted = reservation.is_ok();
            barrier.wait();
            barrier.wait();
            accepted
        }));
    }
    barrier.wait();
    assert_eq!(total(&budget), 2 * charge);
    let bob = budget
        .reserve_with_available("bob", "bob.data", 1, u64::MAX)
        .unwrap();
    assert!(
        budget
            .reserve_with_available("charlie", "c.data", 1, u64::MAX)
            .is_err()
    );
    barrier.wait();
    assert_eq!(
        threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .filter(|accepted| *accepted)
            .count(),
        2
    );
    drop(bob);
    assert_eq!(total(&budget), 0);
}
#[test]
fn recovery_counts_retained_generations_failed_unknown_and_orphan_bytes_once() {
    let root = tempfile::tempdir().unwrap();
    let config = limits(8 * METADATA_ALLOWANCE, 8 * METADATA_ALLOWANCE);
    let budget = SpoolBudget::recover(root.path(), config.clone()).unwrap();
    retained(&budget, "alice", "old.data", 20, b"abc");
    let old = record("old.data", "alice");
    uploads::persist(root.path(), &old).unwrap();
    uploads::retain_generation(root.path(), &old).unwrap();
    uploads::retain_generation(root.path(), &old).unwrap();
    retained(&budget, "alice", "new.data", 20, b"failure");
    let mut new = record("new.data", "alice");
    new.generation = 2;
    new.state = "checksum_mismatch".into();
    uploads::persist(root.path(), &new).unwrap();
    drop(budget);
    fs::write(root.path().join("unknown.data"), b"unknown").unwrap();
    fs::write(root.path().join("broken.json"), b"{broken").unwrap();
    let budget = SpoolBudget::recover(root.path(), config).unwrap();
    assert_eq!(total(&budget), 4 * METADATA_ALLOWANCE + 3 + 7 + 7 + 7);
    assert_eq!(budget.ledger.lock().unwrap().files.len(), 4);
    assert_eq!(fs::read(root.path().join("old.data")).unwrap(), b"abc");
    assert_eq!(
        fs::read(root.path().join("unknown.data")).unwrap(),
        b"unknown"
    );
}
#[test]
fn unknown_owner_is_not_an_account_quota_escape_and_overbudget_startup_stays_readable() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("unattributed.data"), b"unknown").unwrap();
    let budget = SpoolBudget::recover(
        root.path(),
        limits(METADATA_ALLOWANCE, 4 * METADATA_ALLOWANCE),
    )
    .unwrap();
    assert!(
        budget
            .reserve_with_available("alice", "new.data", 0, u64::MAX)
            .is_err()
    );
    assert!(
        budget
            .reserve_with_available("bob", "new.data", 0, u64::MAX)
            .is_err()
    );
    assert_eq!(
        fs::read(root.path().join("unattributed.data")).unwrap(),
        b"unknown"
    );
}
#[test]
fn headroom_accounts_for_concurrent_reservations_without_filling_a_disk() {
    let root = tempfile::tempdir().unwrap();
    let budget = SpoolBudget::recover(
        root.path(),
        limits(4 * METADATA_ALLOWANCE, 4 * METADATA_ALLOWANCE),
    )
    .unwrap();
    let charge = METADATA_ALLOWANCE + 5;
    assert!(
        budget
            .reserve_with_available("alice", "one.data", 5, charge)
            .is_err()
    );
    let first = budget
        .reserve_with_available("alice", "one.data", 5, charge + 1)
        .unwrap();
    assert!(
        budget
            .reserve_with_available("bob", "two.data", 5, 2 * charge)
            .is_err()
    );
    assert!(
        budget
            .reserve_with_available("bob", "two.data", 5, 2 * charge + 1)
            .is_ok()
    );
    drop(first);
}
#[test]
fn directory_has_one_inventory_owner_and_aliases_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let config = limits(4 * METADATA_ALLOWANCE, 4 * METADATA_ALLOWANCE);
    let budget = SpoolBudget::recover(root.path(), config.clone()).unwrap();
    assert!(SpoolBudget::recover(root.path(), config.clone()).is_err());
    drop(budget);
    fs::write(root.path().join("one.data"), b"abc").unwrap();
    fs::hard_link(root.path().join("one.data"), root.path().join("alias.data")).unwrap();
    assert!(SpoolBudget::recover(root.path(), config).is_err());
}
#[test]
fn sidecar_only_crash_remains_charged_and_rebuilds_without_a_payload() {
    let root = tempfile::tempdir().unwrap();
    let config = limits(2 * METADATA_ALLOWANCE, 2 * METADATA_ALLOWANCE);
    let budget = SpoolBudget::recover(root.path(), config.clone()).unwrap();
    let reservation = budget
        .reserve_with_available("alice", "missing.data", 10, u64::MAX)
        .unwrap();
    reservation.persist_owner().unwrap();
    drop(reservation);
    assert_eq!(total(&budget), METADATA_ALLOWANCE);
    drop(budget);
    let budget = SpoolBudget::recover(root.path(), config).unwrap();
    assert_eq!(total(&budget), METADATA_ALLOWANCE);
}
