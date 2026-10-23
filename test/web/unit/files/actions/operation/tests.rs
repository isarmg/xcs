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
        for (authenticated, session_transition) in [
            (false, crate::ui::SessionTransition::Idle),
            (true, crate::ui::SessionTransition::LoggingOut),
            (true, crate::ui::SessionTransition::Restoring),
        ] {
            let mut initial = State {
                authenticated,
                session_transition,
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

#[test]
fn poll_projection_copies_only_unpolled_operation_ids_and_keeps_session_epoch() {
    let mut state = State {
        authenticated: true,
        session_epoch: 17,
        ..Default::default()
    };
    state.files.file_history = json!({"nodes":[{"note":"x".repeat(100_000)}]}).into();
    state.monitor.history = json!({"samples":[{"extra":"y".repeat(100_000)}]}).into();
    state.files.operations = vec![
        json!({"operation_id":"pending","commit_state":"IN_DOUBT"}),
        json!({"operation_id":"in-flight","commit_state":"IN_DOUBT"}),
        json!({"operation_id":"done","commit_state":"PUBLISHED"}),
        json!({"commit_state":"IN_DOUBT"}),
    ];
    state.files.polling_operations.push("in-flight".into());
    crate::snapshot::CLONES.with(|c| c.set(0));
    assert_eq!(poll_work(&state), Some((17, vec!["pending".into()])));
    assert_eq!(
        crate::snapshot::CLONES.with(|c| c.get()),
        0,
        "polling must not clone State or its history snapshots"
    );
    state.session_transition = crate::ui::SessionTransition::LoggingOut;
    assert_eq!(poll_work(&state), None);
}
