#[test]
fn smb_binding_revision_is_required_and_uses_canonical_decimal_strings() {
    let value = json_codec::json!({
        "repository_id":"repo", "name":"Repository", "history_tree_id":"history",
        "main_branch_id":"branch", "primary_slot_id":"P01", "target_copies":1,
        "name_policy":"PortableName", "unicode_version":"16.0.0",
        "smb_enabled":true, "smb_binding_generation":"1"
    });
    let repository: super::Repository = json_codec::from_value(value.clone()).unwrap();
    assert_eq!(repository.smb_binding_generation, 1);
    assert_eq!(
        json_codec::to_value(&repository).unwrap()["smb_binding_generation"],
        "1"
    );
    for invalid in [
        json_codec::json!(null),
        json_codec::json!(1),
        json_codec::json!(""),
        json_codec::json!("01"),
        json_codec::json!("-1"),
        json_codec::json!("18446744073709551616"),
    ] {
        let mut body = value.clone();
        body["smb_binding_generation"] = invalid;
        assert!(json_codec::from_value::<super::Repository>(body).is_err());
    }
    for field in ["smb_enabled", "smb_binding_generation"] {
        let mut absent = value.clone();
        absent.as_object_mut().unwrap().remove(field);
        assert!(
            json_codec::from_value::<super::Repository>(absent).is_err(),
            "{field}"
        );
    }
}

#[test]
fn repository_contract_rejects_removed_account_and_acl_fields() {
    let value = json_codec::json!({
        "repository_id":"repo", "name":"Repository", "history_tree_id":"history",
        "main_branch_id":"branch", "primary_slot_id":"P01", "target_copies":1,
        "name_policy":"PortableName", "unicode_version":"16.0.0",
        "smb_enabled":true, "smb_binding_generation":"1"
    });
    let repository: super::Repository = json_codec::from_value(value.clone()).unwrap();
    let encoded = json_codec::to_value(repository).unwrap();
    assert!(encoded.get("grants").is_none());
    assert!(encoded.get("account_scope_repository_id").is_none());
    for (field, old) in [
        (
            "grants",
            json_codec::json!({"read":[],"write":[],"manage":[]}),
        ),
        ("account_scope_repository_id", json_codec::Value::Null),
    ] {
        let mut removed = value.clone();
        removed[field] = old;
        assert!(json_codec::from_value::<super::Repository>(removed).is_err());
    }
}
