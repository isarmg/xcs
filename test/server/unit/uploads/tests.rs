use super::*;
fn test_budget(root: &Path) -> std::sync::Arc<crate::spool_budget::SpoolBudget> {
    crate::spool_budget::SpoolBudget::recover(
        root,
        crate::SpoolLimits {
            bytes_per_account: 4 * crate::spool_budget::METADATA_ALLOWANCE,
            bytes_total: 4 * crate::spool_budget::METADATA_ALLOWANCE,
            recovery_reserve_bytes: 1,
        },
    )
    .unwrap()
}
#[test]
fn staging_guard_preserves_unpublished_bytes_until_and_after_last_job() {
    let root = tempfile::tempdir().unwrap();
    let budget = test_budget(root.path());
    let path = root.path().join("unpublished.data");
    let reservation = budget.reserve("alice", "unpublished.data", 10).unwrap();
    let guard = StagedFile::open_budgeted(path.clone(), false, reservation).unwrap();
    let running_job = guard.clone();
    drop(guard);
    assert!(budget.reserve("alice", "unpublished.data", 10).is_err());
    running_job.write(b"partial").unwrap();
    drop(running_job);
    assert_eq!(fs::read(path).unwrap(), b"partial");
    assert!(budget.reserve("alice", "unpublished.data", 10).is_ok());
}

#[test]
fn uncertain_directory_sync_retains_bytes_and_blocks_fresh_admission_until_inventory() {
    let root = tempfile::tempdir().unwrap();
    let previous = root.path().join("previous.data");
    fs::write(&previous, b"previous").unwrap();
    let budget = test_budget(root.path());
    let path = root.path().join("new.data");
    let reservation = budget.reserve("alice", "new.data", 9).unwrap();
    let guard = StagedFile::open_budgeted(path.clone(), false, reservation).unwrap();
    guard.write(b"new bytes").unwrap();
    guard.sync().unwrap();
    let record = UploadRecord {
        workspace_id: "ws".into(),
        item_id: "item".into(),
        principal: "alice".into(),
        path: "file".into(),
        generation: 1,
        logical_size: 9,
        content_hash: Some(blake3::hash(b"new bytes").to_hex().to_string()),
        persisted_bytes: 9,
        persisted_hash: blake3::hash(b"new bytes").to_hex().to_string(),
        state: "incomplete".into(),
        data_file: "new.data".into(),
    };
    assert!(
        persist_staged_with_sync(root.path(), &record, &guard, || {
            Err(ApiError::io(std::io::Error::other(
                "injected directory sync failure",
            )))
        })
        .is_err()
    );
    assert_eq!(
        load(root.path(), "ws", "item").unwrap().unwrap().data_file,
        "new.data"
    );
    drop(guard);
    assert_eq!(fs::read(&path).unwrap(), b"new bytes");
    assert_eq!(fs::read(&previous).unwrap(), b"previous");
    assert!(budget.reserve("bob", "next.data", 1).is_err());
    drop(budget);
    let budget = test_budget(root.path());
    assert!(budget.reserve("alice", "new.data", 9).is_ok());
    assert_eq!(fs::read(&path).unwrap(), b"new bytes");
    assert_eq!(fs::read(&previous).unwrap(), b"previous");
}

#[test]
fn interrupted_upload_retains_and_checks_its_persisted_prefix() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("data"), b"original partial").unwrap();
    let record = UploadRecord {
        workspace_id: "ws".into(),
        item_id: "item".into(),
        principal: "alice".into(),
        path: "file".into(),
        generation: 1,
        logical_size: 30,
        content_hash: Some("0".repeat(64)),
        persisted_bytes: 16,
        persisted_hash: blake3::hash(b"original partial").to_hex().to_string(),
        state: "incomplete".into(),
        data_file: "data".into(),
    };
    persist(dir.path(), &record).unwrap();
    let loaded = load(dir.path(), "ws", "item").unwrap().unwrap();
    assert_eq!(loaded.persisted_bytes, 16);
    let mut prefix = ExistingPrefix::open(dir.path(), &loaded).unwrap().unwrap();
    assert!(prefix.check(b"different partial").is_err());
    let mut prefix = ExistingPrefix::open(dir.path(), &loaded).unwrap().unwrap();
    prefix.check(b"original partial and more").unwrap();
    assert!(prefix.complete());
    assert_eq!(status(&loaded)["state"], "RECEIVING");
}

#[test]
fn corrupt_or_oversized_progress_is_an_internal_state_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(format!("{}.json", key("ws", "item")));
    for bytes in [
        b"{broken".to_vec(),
        vec![b' '; 16_385],
        vec![b' '; 1024 * 1024],
    ] {
        fs::write(&path, bytes).unwrap();
        let error = load(dir.path(), "ws", "item").err().unwrap();
        assert_eq!(error.status, 500);
        assert_eq!(error.code, "UPLOAD_STATE_INVALID");
    }
}

#[test]
fn failed_atomic_progress_publication_cleans_up_its_temporary_file() {
    let dir = tempfile::tempdir().unwrap();
    let record = UploadRecord {
        workspace_id: "ws".into(),
        item_id: "item".into(),
        principal: "alice".into(),
        path: "file".into(),
        generation: 1,
        logical_size: 0,
        content_hash: Some(blake3::hash(b"").to_hex().to_string()),
        persisted_bytes: 0,
        persisted_hash: blake3::hash(b"").to_hex().to_string(),
        state: "incomplete".into(),
        data_file: "data".into(),
    };
    // A directory at the destination makes rename fail deterministically,
    // even when the test runs as root and chmod cannot deny writes.
    fs::create_dir(dir.path().join(format!("{}.json", key("ws", "item")))).unwrap();
    assert!(persist(dir.path(), &record).is_err());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}
