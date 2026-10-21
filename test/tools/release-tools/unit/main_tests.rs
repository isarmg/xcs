use super::*;
#[test]
fn canonical_semver_only() {
    for tag in [
        "v0.1.0",
        "v1.2.3-rc.1",
        "v2.0.0+build.3",
        "v1.0.0-beta.2+abc",
    ] {
        assert!(tag_version(tag).is_ok(), "{tag}");
    }
    for tag in [
        "1.0.0",
        "v1.0",
        "v01.0.0",
        "v1.0.0-01",
        "v1.0.0\n",
        "v1.0.0;echo bad",
        "v1.0.0-",
    ] {
        assert!(tag_version(tag).is_err(), "{tag}");
    }
}
#[test]
fn real_manifests_match_workspace() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let workspace = toml_file(&root.join("Cargo.toml")).unwrap();
    let version = workspace["workspace"]["package"]["version"]
        .as_str()
        .unwrap();
    assert!(validate(&root, &format!("v{version}")).is_ok());
    assert!(validate(&root, "v999999.0.0").is_err());
}
