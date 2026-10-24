use super::*;
#[test]
fn new_repository_defaults_to_one_actual_copy_and_preserves_explicit_choices() {
    let mut fields = initial_fields("repository");
    fields.insert("name".into(), "one disk".into());
    fields.insert("slot".into(), "P01".into());
    let (_, path, body) = build("repository", &fields, "operation").unwrap();
    assert_eq!(path, "/api/repositories");
    assert_eq!(body["target_copies"], 1);
    fields.insert("copies".into(), "3".into());
    assert_eq!(
        build("repository", &fields, "operation").unwrap().2["target_copies"],
        3
    );
    fields.insert("copies".into(), "0".into());
    assert!(build("repository", &fields, "operation").is_err());
    assert_eq!(initial_fields("policy")["copies"], "1");
    assert!(
        initial_fields("save").is_empty(),
        "historical saves retain inherited policy unless explicitly overridden"
    );
}

#[test]
fn retired_account_and_acl_actions_cannot_build_requests() {
    for kind in ["account", "grants"] {
        assert!(build(kind, &Fields::new(), "operation").is_err());
    }
}

#[test]
fn storage_mutations_require_a_secure_operation_identity() {
    for kind in [
        "repository",
        "smb",
        "policy",
        "rule",
        "branch",
        "link",
        "purge",
        "split",
        "save",
    ] {
        assert!(build(kind, &Fields::new(), "").is_err());
    }
}
