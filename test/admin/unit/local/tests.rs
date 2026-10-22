use super::*;

fn registration(root: &Path, config: std::path::PathBuf) -> Args {
    Args {
        root: root.join("system"),
        principal: "admin".into(),
        command: Command::RegisterDisk {
            config,
            slot: "D01".into(),
            disk_id: storage_engine::new_id("disk"),
            path: root.join("fourth"),
            level: 1,
            upstream: None,
            metadata_member: false,
        },
    }
}
#[tokio::test]
async fn registering_a_disk_keeps_an_exact_backup_and_atomically_replaces_the_draft() {
    let root = tempfile::tempdir().unwrap();
    let draft = Config::development([
        root.path().join("p"),
        root.path().join("b"),
        root.path().join("c"),
    ]);
    let path = root.path().join("config.json");
    let original = json_codec::to_vec(&draft).unwrap();
    private_io::write_new(&path, &original).unwrap();
    run(registration(root.path(), path.clone())).await.unwrap();
    let saved: Config = storage_engine::decode_strict(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(saved.disks.len(), 4);
    assert_eq!(saved.disks.last().unwrap().slot_id, "D01");
    assert_eq!(
        std::fs::read(path.with_extension("json.before-registration")).unwrap(),
        original
    );
}
#[tokio::test]
async fn different_configuration_extensions_keep_independent_registration_backups() {
    let root = tempfile::tempdir().unwrap();
    let draft = Config::development([
        root.path().join("p"),
        root.path().join("b"),
        root.path().join("c"),
    ]);
    let first = root.path().join("config.json");
    let second = root.path().join("config.draft");
    let original = json_codec::to_vec(&draft).unwrap();
    private_io::write_new(&first, &original).unwrap();
    let mut other = draft.clone();
    other.pool_id = storage_engine::new_id("pool");
    let other_original = json_codec::to_vec(&other).unwrap();
    private_io::write_new(&second, &other_original).unwrap();
    run(registration(root.path(), first)).await.unwrap();
    run(registration(root.path(), second)).await.unwrap();
    assert_eq!(
        std::fs::read(root.path().join("config.json.before-registration")).unwrap(),
        original
    );
    assert_eq!(
        std::fs::read(root.path().join("config.draft.before-registration")).unwrap(),
        other_original
    );
}

#[test]
fn registration_backup_preserves_the_complete_native_file_name() {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};
    for name in ["config", "config.json", "config.draft", ".config"] {
        assert_eq!(
            registration_backup(Path::new(name)).unwrap(),
            Path::new(&format!("{name}.before-registration"))
        );
    }
    let name = std::ffi::OsString::from_vec(b"config-\xff.json".to_vec());
    assert_eq!(
        registration_backup(Path::new(&name))
            .unwrap()
            .as_os_str()
            .as_bytes(),
        b"config-\xff.json.before-registration"
    );
    assert!(registration_backup(Path::new("/")).is_err());
}

#[tokio::test]
async fn oversized_backup_name_fails_without_changing_the_draft() {
    let root = tempfile::tempdir().unwrap();
    let draft = Config::development([
        root.path().join("p"),
        root.path().join("b"),
        root.path().join("c"),
    ]);
    let path = root.path().join("c".repeat(250));
    let original = json_codec::to_vec(&draft).unwrap();
    private_io::write_new(&path, &original).unwrap();
    let error = run(registration(root.path(), path.clone()))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("draft was not changed"));
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
}

#[tokio::test]
async fn non_utf8_registration_path_is_rejected_before_any_write() {
    use std::os::unix::ffi::OsStringExt;
    let root = tempfile::tempdir().unwrap();
    let draft = Config::development([
        root.path().join("p"),
        root.path().join("b"),
        root.path().join("c"),
    ]);
    let path = root
        .path()
        .join(std::ffi::OsString::from_vec(b"config-\xff.json".to_vec()));
    let original = json_codec::to_vec(&draft).unwrap();
    private_io::write_new(&path, &original).unwrap();
    let error = run(registration(root.path(), path.clone()))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("valid UTF-8"));
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
}

#[tokio::test]
async fn repeated_registration_keeps_the_immediately_previous_draft() {
    let root = tempfile::tempdir().unwrap();
    let draft = Config::development([
        root.path().join("p"),
        root.path().join("b"),
        root.path().join("c"),
    ]);
    let path = root.path().join("config.json");
    private_io::write_new(&path, &json_codec::to_vec(&draft).unwrap()).unwrap();
    run(registration(root.path(), path.clone())).await.unwrap();
    let previous = std::fs::read(&path).unwrap();
    let mut args = registration(root.path(), path.clone());
    if let Command::RegisterDisk { slot, path, .. } = &mut args.command {
        *slot = "D02".into();
        *path = root.path().join("fifth");
    }
    run(args).await.unwrap();
    assert_eq!(
        std::fs::read(registration_backup(&path).unwrap()).unwrap(),
        previous
    );
}

#[tokio::test]
async fn invalid_upstream_level_is_an_error_and_never_modifies_the_draft() {
    let root = tempfile::tempdir().unwrap();
    let mut draft = Config::development([
        root.path().join("p"),
        root.path().join("b"),
        root.path().join("c"),
    ]);
    draft.disks[0].level = u32::MAX;
    let path = root.path().join("config.json");
    let original = json_codec::to_vec(&draft).unwrap();
    private_io::write_new(&path, &original).unwrap();
    assert!(run(registration(root.path(), path.clone())).await.is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert!(!path.with_extension("json.before-registration").exists());
}
#[tokio::test]
async fn unsafe_password_input_is_rejected_before_pool_or_disk_initialization() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let password_file = root.path().join("password");
    private_io::write_new(&password_file, b"fixture-password").unwrap();
    std::fs::set_permissions(&password_file, std::fs::Permissions::from_mode(0o644)).unwrap();
    let system = root.path().join("system");
    let disks = [
        root.path().join("p"),
        root.path().join("b"),
        root.path().join("c"),
    ];
    let args = Args {
        root: system.clone(),
        principal: "admin".into(),
        command: Command::Init {
            config: None,
            development_disks: disks.to_vec(),
            username: "admin".into(),
            password_file: Some(password_file),
        },
    };
    assert!(run(args).await.is_err());
    assert!(!system.exists());
    assert!(disks.iter().all(|path| !path.exists()));
}
