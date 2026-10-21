use super::*;
use std::io::{Seek, SeekFrom};

fn disk(temp: &tempfile::TempDir) -> DiskConfig {
    Config::development([
        temp.path().join("one"),
        temp.path().join("two"),
        temp.path().join("three"),
    ])
    .disks
    .remove(0)
}
fn put(d: &DiskConfig, kind: &str, sequence: u64) -> Receipt {
    append(
        d,
        kind,
        sequence,
        &format!("{sequence:064x}"),
        &"b".repeat(64),
    )
    .unwrap()
}

#[test]
fn unnumbered_frame_header_binds_length_payload_and_receipt() {
    let temp = tempfile::tempdir().unwrap();
    let d = disk(&temp);
    let receipt = put(&d, "PREPARE", 1);
    let bytes = std::fs::read(log_path(&d)).unwrap();
    let payload = canonical_json(&receipt.record).unwrap();
    assert_eq!(&bytes[..8], LOG_MAGIC);
    assert_eq!(
        u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize,
        payload.len()
    );
    assert_eq!(bytes.len(), LOG_HEADER_BYTES + payload.len() + 32);
    assert_eq!(
        &bytes[LOG_HEADER_BYTES..LOG_HEADER_BYTES + payload.len()],
        payload
    );
    assert!(
        receipt_binds(
            &receipt,
            "PREPARE",
            1,
            &format!("{:064x}", 1),
            &"b".repeat(64)
        )
        .unwrap()
    );
    assert_eq!(scan(&d).unwrap().valid_bytes, bytes.len() as u64);
    let mut invalid = bytes;
    invalid[..8].copy_from_slice(b"ZXSLOG2\0");
    std::fs::write(log_path(&d), &invalid).unwrap();
    assert_eq!(scan(&d).err().unwrap().code(), "LOG_CORRUPT");
    invalid[..8].copy_from_slice(LOG_MAGIC);
    invalid[8..12].copy_from_slice(&((MAX_FRAME + 1) as u32).to_le_bytes());
    std::fs::write(log_path(&d), invalid).unwrap();
    assert!(scan(&d).is_err());
}

#[test]
fn latest_receipts_are_idempotent_and_old_retries_rescan() {
    let temp = tempfile::tempdir().unwrap();
    let d = disk(&temp);
    let first = put(&d, "PREPARE", 1);
    let latest = put(&d, "ACCEPT", 1);
    assert_eq!(put(&d, "PREPARE", 1).frame_hash, first.frame_hash);
    assert_eq!(put(&d, "ACCEPT", 1).frame_hash, latest.frame_hash);
    put(&d, "PREPARE", 2);
    let len = std::fs::metadata(log_path(&d)).unwrap().len();
    assert_eq!(put(&d, "PREPARE", 1).frame_hash, first.frame_hash);
    assert_eq!(std::fs::metadata(log_path(&d)).unwrap().len(), len);
    assert_eq!(scan(&d).unwrap().records.len(), 3);
    assert_eq!(
        append(&d, "FINALIZE", 1, &format!("{:064x}", 1), &"b".repeat(64))
            .unwrap_err()
            .code,
        "LOG_FORK"
    );
}

#[test]
fn cached_append_detects_same_length_prefix_corruption() {
    let temp = tempfile::tempdir().unwrap();
    let d = disk(&temp);
    put(&d, "PREPARE", 1);
    put(&d, "PREPARE", 2);
    let path = log_path(&d);
    let old = std::fs::metadata(&path).unwrap();
    let mut file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.seek(SeekFrom::Start(20)).unwrap();
    file.write_all(b"!").unwrap();
    file.set_times(std::fs::FileTimes::new().set_modified(old.modified().unwrap()))
        .unwrap();
    file.sync_all().unwrap();
    assert_eq!(
        append(&d, "ACCEPT", 2, &format!("{:064x}", 2), &"b".repeat(64))
            .unwrap_err()
            .code,
        "LOG_CORRUPT"
    );
}

#[test]
fn cached_append_repairs_torn_external_tail() {
    let temp = tempfile::tempdir().unwrap();
    let d = disk(&temp);
    put(&d, "PREPARE", 1);
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(log_path(&d))
        .unwrap();
    file.write_all(b"ZXSLO").unwrap();
    file.sync_all().unwrap();
    put(&d, "ACCEPT", 1);
    let scanned = scan(&d).unwrap();
    assert!(!scanned.torn_tail);
    assert_eq!(scanned.records.len(), 2);
}

#[test]
fn replacement_and_truncation_invalidate_cached_receipts() {
    let temp = tempfile::tempdir().unwrap();
    let d = disk(&temp);
    put(&d, "PREPARE", 1);
    let old = put(&d, "ACCEPT", 1);
    let path = log_path(&d);
    std::fs::rename(&path, path.with_extension("old")).unwrap();
    let fresh = put(&d, "ACCEPT", 1);
    assert_ne!(fresh.frame_hash, old.frame_hash);
    assert_eq!(scan(&d).unwrap().records.len(), 1);
    std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(0)
        .unwrap();
    put(&d, "PREPARE", 2);
    let scanned = scan(&d).unwrap();
    assert_eq!(scanned.records.len(), 1);
    assert_eq!(scanned.records[0].0.sequence, 2);
}

#[test]
fn cold_append_revalidates_chain_and_cache_retains_only_latest_slot() {
    // Keep this key after ordinary .tmp* fixtures in deterministic eviction
    // order, so unrelated parallel tests cannot evict this measured tail.
    let temp = tempfile::Builder::new()
        .prefix("zzzz-tail-")
        .tempdir()
        .unwrap();
    let d = disk(&temp);
    let before = SCAN_COUNT.with(|count| count.get());
    for sequence in 1..=40 {
        put(&d, "PREPARE", sequence);
        put(&d, "ACCEPT", sequence);
    }
    assert_eq!(
        SCAN_COUNT.with(|count| count.get()) - before,
        1,
        "ordinary appends must scan only the initial cold journal"
    );
    {
        let mut cache = tails().lock().unwrap();
        let tail = cache.remove(&log_path(&d)).unwrap();
        assert_eq!(tail.records.len(), 2);
        assert_eq!(tail.records.capacity(), 2);
        assert!(tail.records.iter().all(|(r, _)| r.sequence == 40));
    }
    put(&d, "FINALIZE", 40);
    assert_eq!(
        SCAN_COUNT.with(|count| count.get()) - before,
        2,
        "evicted/cold state must verify the full journal again"
    );
    assert_eq!(scan(&d).unwrap().records.len(), 81);
}
