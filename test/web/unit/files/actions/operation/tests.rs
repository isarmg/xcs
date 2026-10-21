use super::*;

#[test]
fn missing_or_unrelated_receipts_keep_the_original_mutation_pending() {
    Owner::new().with(|| {
        for response in [
            Value::Null,
            json!({}),
            json!({"state":"SUCCEEDED"}),
            json!({"operation_id":"other","state":"SUCCEEDED"}),
            json!({"operation":{"operation_id":"other","state":"SUCCEEDED"}}),
        ] {
            let state = RwSignal::new(State::default());
            assert_eq!(
                record_receipt(state, "original", response),
                SaveStatus::Unknown
            );
            state.with_untracked(|s| {
                assert!(super::super::pending(s));
                assert_eq!(s.files.operations.len(), 1);
                assert_eq!(s.files.operations[0]["operation_id"], "original");
            });
            // A subsequent poll settles the same ID, rather than creating
            // another operation or leaving a permanent phantom lock.
            assert_eq!(
                record_receipt(
                    state,
                    "original",
                    json!({
                        "operation_id":"original", "state":"SUCCEEDED"
                    })
                ),
                SaveStatus::Published
            );
            state.with_untracked(|s| {
                assert!(!super::super::pending(s));
                assert_eq!(s.files.operations.len(), 1);
            });
        }
    });
}

#[test]
fn matching_wrapped_receipts_preserve_rejection_and_publication_evidence() {
    for status in ["REJECTED", "PUBLISHED", "PUBLISHED_DEGRADED", "IN_DOUBT"] {
        let value = json!({"operation":{"operation_id":"original","commit_state":status}});
        assert_eq!(receipt("original", value.clone()), value);
    }
}
