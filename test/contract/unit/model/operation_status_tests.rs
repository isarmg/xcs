use super::*;

// These strings are the existing public commit-state vocabulary, plus the
// persisted protection values emitted by normal and recovery publication.
const COMMIT_STATES: &[&str] = &[
    "STAGING",
    "SEALED",
    "PREPARED",
    "DECIDING",
    "DECIDED",
    "PROTECTED",
    "PUBLISHED",
    "ACKED",
    "RECOVERING",
    "RECOVERY_REQUIRED",
    "IN_DOUBT",
    "PUBLISHED_DEGRADED",
    "ABORTED",
];
const PROTECTION_STATES: &[&str] = &[
    "NOT_APPLICABLE",
    "TARGET_MET",
    "BELOW_TARGET",
    "UNCONFIRMED",
];

fn frozen_operation(commit: &str, protection: &str) -> Vec<u8> {
    format!(r#"{{"branch_id":null,"commit_id":null,"commit_state":"{commit}","current_health":"HEALTHY","history_tree_id":null,"kind":"workspace.commit","operation_id":"operation_fixture","protection_at_publish":"{protection}","request_digest":"fixture","result":{{}},"sequence":"18446744073709551615"}}"#).into_bytes()
}

#[test]
fn operation_status_rejects_unknown_or_malformed_values() {
    for (commit, protection) in [
        ("published", "TARGET_MET"),
        ("PUBLISHED_TYPO", "TARGET_MET"),
        ("", "TARGET_MET"),
        ("PUBLISHED", "HEALTHY"),
        ("PUBLISHED", "target_met"),
        ("PUBLISHED", ""),
    ] {
        assert!(
            crate::decode_strict::<OperationResult>(&frozen_operation(commit, protection)).is_err()
        );
    }
    for invalid in ["null", "0", "{}", "[]"] {
        assert!(serde_json::from_str::<CommitState>(invalid).is_err());
        assert!(serde_json::from_str::<ProtectionAtPublish>(invalid).is_err());
    }
}

#[test]
fn operation_status_preserves_frozen_bytes_and_digest() {
    for commit in COMMIT_STATES {
        for protection in PROTECTION_STATES {
            let bytes = frozen_operation(commit, protection);
            let operation: OperationResult = crate::decode_strict(&bytes).unwrap();
            assert_eq!(crate::canonical_json(&operation).unwrap(), bytes);
            let mut expected = blake3::Hasher::new();
            expected.update(b"zxs:state-page\0");
            expected.update(&bytes);
            assert_eq!(
                crate::object_hash("state-page", &operation).unwrap(),
                expected.finalize().to_hex().as_str()
            );
        }
    }
}
