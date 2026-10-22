use super::*;
#[test]
fn file_sizes_require_exact_browser_integer_sizes() {
    validate_size(0).unwrap();
    validate_size(5 * 1024 * 1024 * 1024).unwrap();
    validate_size(9_007_199_254_740_991).unwrap();
    assert_eq!(
        validate_size(9_007_199_254_740_992).unwrap_err().code,
        "DOWNLOAD_SIZE_UNSUPPORTED"
    );
    assert_eq!(
        validate_size(u64::MAX).unwrap_err().code,
        "DOWNLOAD_SIZE_UNSUPPORTED"
    );
}
#[test]
fn large_stream_uses_reused_small_buffer() {
    let chunk = [0u8; 64 * 1024];
    let mut state = Integrity::new(65 * 1024 * 1024);
    for _ in 0..1040 {
        state.update(&chunk).unwrap();
    }
    assert_eq!(state.received, 65 * 1024 * 1024);
    state
        .finish("c7d5e9a4a234b48a92a436fcb54145065325e8a17b0d2f5aced359fa505a95f7")
        .unwrap();
}
#[test]
fn incremental_integrity_accepts_empty_and_arbitrary_boundaries() {
    let bytes = vec![17; 1024 * 1024 + 17];
    let mut state = Integrity::new(bytes.len() as u64);
    for chunk in bytes.chunks(997) {
        state.update(chunk).unwrap();
    }
    state
        .finish(blake3::hash(&bytes).to_hex().as_str())
        .unwrap();
    Integrity::new(0)
        .finish(blake3::hash(b"").to_hex().as_str())
        .unwrap();
}
#[test]
fn integrity_rejects_corruption_truncation_and_overrun() {
    let mut state = Integrity::new(4);
    state.update(b"abc").unwrap();
    assert_eq!(
        state
            .finish(blake3::hash(b"abc").to_hex().as_str())
            .unwrap_err()
            .code,
        "CHECKSUM_MISMATCH"
    );
    assert!(state.update(b"de").is_err());
    state.update(b"d").unwrap();
    assert!(
        state
            .finish(blake3::hash(b"abce").to_hex().as_str())
            .is_err()
    );
    state
        .finish(blake3::hash(b"abcd").to_hex().as_str())
        .unwrap();
}

#[test]
fn foreground_cancellation_reports_pending_cleanup_without_claiming_rollback() {
    assert!(foreground_result(None, None, false).is_none());
    let cancelled = foreground_result(None, None, true).unwrap().unwrap_err();
    assert_eq!(cancelled.code, "DOWNLOAD_CANCELLED");
    assert!(
        cancelled.message.contains("cleanup is pending")
            || cancelled.message.contains("清理尚未完成")
    );
    assert!(
        cancelled.message.contains("may still finish") || cancelled.message.contains("仍可能完成")
    );
}
#[test]
fn foreground_preserves_failure_and_completed_result_over_cancellation() {
    let failure = foreground_result(None, Some(mismatch()), true)
        .unwrap()
        .unwrap_err();
    assert_eq!(failure.code, "CHECKSUM_MISMATCH");
    let saved = foreground_result(Some(Ok("verified".to_owned())), None, true)
        .unwrap()
        .unwrap();
    assert_eq!(saved, "verified");
}

#[test]
fn retained_copy_label_does_not_disclose_previous_session_filename() {
    let sensitive_name = "private-client-report.pdf";
    assert_eq!(spool::retained_label(sensitive_name, true), sensitive_name);
    assert!(!spool::retained_label(sensitive_name, false).contains(sensitive_name));
}

#[test]
fn download_observer_is_single_scoped_and_called_outside_its_borrow() {
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let first = observe(move || count.set(count.get() + 1));
    notify_now();
    assert_eq!(calls.get(), 1);
    let count = calls.clone();
    let second = observe(move || {
        OBSERVER.with(|slot| assert!(slot.borrow().is_some()));
        count.set(count.get() + 10);
    });
    drop(first);
    notify_now();
    assert_eq!(calls.get(), 11);
    drop(second);
    notify_now();
    assert_eq!(calls.get(), 11);
}
