use super::{Grants, RepositoryAccess};

#[test]
fn capabilities_are_independent_and_manage_implies_both() {
    for bits in 0..8 {
        let mut grants = Grants::default();
        if bits & 1 != 0 {
            grants.read.insert("user".into());
        }
        if bits & 2 != 0 {
            grants.write.insert("user".into());
        }
        if bits & 4 != 0 {
            grants.manage.insert("user".into());
        }
        for (access, expected) in [
            (RepositoryAccess::Read, bits & 5 != 0),
            (RepositoryAccess::Write, bits & 6 != 0),
            (RepositoryAccess::Manage, bits & 4 != 0),
        ] {
            assert_eq!(grants.allows("user", access), expected);
            assert!(!grants.allows("other", access));
        }
    }
}

#[test]
fn smb_binding_revision_is_required_and_uses_canonical_decimal_strings() {
    let value = json_codec::json!({
        "repository_id":"repo", "name":"Repository", "history_tree_id":"history",
        "main_branch_id":"branch", "primary_slot_id":"P01", "target_copies":1,
        "name_policy":"PortableName", "unicode_version":"16.0.0",
        "grants":{"read":[],"write":[],"manage":[]},
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
    let mut absent = value;
    absent
        .as_object_mut()
        .unwrap()
        .remove("smb_binding_generation");
    assert!(json_codec::from_value::<super::Repository>(absent).is_err());
}
