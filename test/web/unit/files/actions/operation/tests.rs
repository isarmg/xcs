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

#[test]
fn metadata_polling_requires_an_active_session_before_scheduling_network_work() {
    Owner::new().with(|| {
        for (authenticated, logout_pending, restoring) in [
            (false, false, false),
            (true, true, false),
            (true, false, true),
        ] {
            let mut initial = State {
                authenticated,
                logout_pending,
                restoring,
                ..Default::default()
            };
            initial
                .files
                .operations
                .push(json!({"operation_id":"pending-rename","state":"in_doubt"}));
            let state = RwSignal::new(initial);
            // Native tests cannot schedule browser requests; returning here also
            // proves that inactive sessions do not admit a metadata poll.
            poll(state);
            state.with_untracked(|s| {
                assert!(s.files.polling_operations.is_empty());
                assert!(super::super::pending(s));
                assert!(!super::super::write_allowed(s));
            });
        }
    });
}
