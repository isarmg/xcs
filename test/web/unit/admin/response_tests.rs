use super::*;
use json_codec::json;
fn contract(path: &str, body: Value) -> Contract {
    Contract::new("POST", path, Some(&body), "original", "admin")
}
fn raw_operation() -> Value {
    json!({"operation_id":"original", "kind":"plan.cancel", "commit_id":null,"history_tree_id":null,"branch_id":null,"commit_state":"PUBLISHED","protection_at_publish":"NOT_APPLICABLE","current_health":"UNKNOWN","request_digest":"digest","sequence":"4","result":{}})
}
fn wire_operation() -> Value {
    json!({"operation_id":"original","operation_kind":"policy.update","commit_state":"PUBLISHED","request_digest":"digest","current_health":{"state":"UNKNOWN","checked_at":"2026-10-22T00:00:00Z"},"result":{},"state":"completed","publication_sequence":"4","protection_at_publish":"NOT_APPLICABLE","error":null})
}
#[test]
fn account_success_is_not_an_operation_receipt() {
    let c = contract("/api/admin/accounts", Value::Null);
    assert!(c.validate(json!({"updated":true})).is_ok());
    for value in [
        Value::Null,
        json!({}),
        json!({"updated":false}),
        json!({"updated":"true"}),
        raw_operation(),
    ] {
        assert!(c.validate(value).unwrap_err().mutation_outcome_unknown());
    }
}
#[test]
fn operation_contracts_reject_missing_wrong_and_nonterminal_receipts() {
    for path in [
        "/api/plans/p/commit",
        "/api/plans/p/cancel",
        "/api/repositories/r/grants",
    ] {
        let c = contract(path, Value::Null);
        assert!(c.validate(raw_operation()).is_ok());
        for value in [Value::Null, json!({}), wire_operation()] {
            assert!(c.validate(value).is_err());
        }
        for (key, bad) in [("operation_id", "unrelated"), ("commit_state", "IN_DOUBT")] {
            let mut value = raw_operation();
            value[key] = bad.into();
            assert!(c.validate(value).is_err());
        }
    }
    for path in [
        "/api/repositories/r/smb-binding",
        "/api/policies/p",
        "/api/primary-placement-rules/r",
    ] {
        let c = contract(path, Value::Null);
        assert!(c.validate(wire_operation()).is_ok());
        assert!(c.validate(raw_operation()).is_err());
        let mut value = wire_operation();
        value["operation_id"] = "unrelated".into();
        assert!(c.validate(value).is_err());
    }
}
#[test]
fn inspection_must_match_original_operation_and_terminal_state() {
    let c = Contract::new("GET", "/api/operations/original", None, "", "admin");
    assert!(c.validate(wire_operation()).is_ok());
    assert!(terminal(&wire_operation(), "original"));
    let mut value = wire_operation();
    value["operation_id"] = "other".into();
    assert!(c.validate(value.clone()).is_err());
    assert!(!terminal(&value, "original"));
    value["operation_id"] = "original".into();
    value["commit_state"] = "IN_DOUBT".into();
    assert!(c.validate(value.clone()).is_ok());
    assert!(!terminal(&value, "original"));
}
#[test]
fn domain_and_preview_contracts_accept_real_shapes_not_generic_receipts() {
    let repository = json!({"repository_id":"r", "name":"Repo", "history_tree_id":"h", "main_branch_id":"b", "primary_slot_id":"P01", "target_copies":1,"name_policy":"portable", "unicode_version":"17", "grants":{"read":[],"write":[],"manage":[]},"smb_binding_generation":"1"});
    let c = contract(
        "/api/repositories",
        json!({"name":"Repo","primary_slot_id":"P01","target_copies":1}),
    );
    assert!(c.validate(repository.clone()).is_ok());
    let mut wrong = repository;
    wrong["name"] = "Other".into();
    assert!(c.validate(wrong).is_err());
    let branch = json!({"branch_id":"b","history_tree_id":"h","name":"Branch","head_commit_id":"new","fork_commit_id":"source","branch_kind":"full","generation":"1","state":"active","pending_file_versions":{},"materialized":true,"inherited":false,"content_state":"LIVE"});
    assert!(
        contract(
            "/api/repositories/r/branches",
            json!({"name":"Branch","source_commit_id":"source"})
        )
        .validate(branch)
        .is_ok()
    );
    let link = json!({"merge_link_id":"m","history_tree_id":"h","source_branch_id":"s","source_commit_id":"sc","target_branch_id":"t","target_commit_id":"tc","author":"admin","operation_id":"original"});
    assert!(
        contract("/api/history-trees/h/merge-links", link.clone())
            .validate(link)
            .is_ok()
    );
    for (suffix, kind) in [
        ("purge-plans", "purge"),
        ("split-plans", "split"),
        ("historical-save-plans", "historical_save"),
    ] {
        let c = contract(&format!("/api/history-trees/h/{suffix}"), Value::Null);
        let plan = json!({"plan_id":"p","kind":kind,"principal":"admin","expected_sequence":"4","digest":"digest","input":{"history_tree_id":"h"},"state":"ready"});
        assert!(c.validate(plan.clone()).is_ok());
        let mut wrong = plan;
        wrong["input"]["history_tree_id"] = "other".into();
        assert!(c.validate(wrong).is_err());
        assert!(c.validate(raw_operation()).is_err());
    }
}

#[test]
fn branch_creation_accepts_wire_projection_without_relaxing_domain_validation() {
    let c = contract(
        "/api/repositories/r/branches",
        json!({"name":"Branch","source_commit_id":"source"}),
    );
    // branch_wire always adds these three fields, including for empty branches.
    let empty = json!({"branch_id":"b","history_tree_id":"h","name":"Branch","head_commit_id":"new","fork_commit_id":"source","branch_kind":"full","generation":"1","state":"active","pending_file_versions":{},"materialized":true,"inherited":false,"content_state":"LIVE"});
    let mut pending = empty.clone();
    pending["pending_file_versions"] = json!({"file":"version"});
    pending["materialized"] = false.into();
    pending["inherited"] = true.into();
    pending["content_state"] = "PENDING".into();
    for valid in [empty, pending] {
        assert!(c.validate(valid.clone()).is_ok());
        for key in ["materialized", "inherited", "content_state"] {
            let mut missing = valid.clone();
            missing.as_object_mut().unwrap().remove(key);
            assert!(c.validate(missing).is_err());
        }
        for (key, bad) in [
            ("materialized", valid["inherited"].clone()),
            ("inherited", valid["materialized"].clone()),
            ("content_state", "UNKNOWN".into()),
            ("generation", json!(1)),
            ("unexpected", true.into()),
        ] {
            let mut malformed = valid.clone();
            malformed[key] = bad;
            assert!(c.validate(malformed).is_err());
        }
    }
}
