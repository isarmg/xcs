use std::process::Command;
#[test]
fn rescue_exposes_only_offline_recovery_and_configuration_operations() {
    let binary = env!("CARGO_BIN_EXE_zxs-rescue");
    for name in [
        "restore-system",
        "recovery-preview",
        "recovery-commit",
        "offline-check",
        "offline-export",
        "package",
        "configuration",
    ] {
        assert!(
            Command::new(binary)
                .args([name, "--help"])
                .output()
                .unwrap()
                .status
                .success()
        );
    }
    for name in [
        "init",
        "init-auth",
        "serve",
        "account",
        "create-repo",
        "export-history",
        "register-disk",
        "config-template",
        "--server",
    ] {
        let output = Command::new(binary).arg(name).output().unwrap();
        assert_eq!(output.status.code(), Some(2));
    }
}
#[test]
fn recovery_requires_explicit_loss_acknowledgement_before_opening_pool() {
    let directory = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_zxs-rescue"))
        .current_dir(directory.path())
        .args(["recovery-commit", "recovery", "--digest", "digest"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--accept-permanent-loss"));
    assert!(!directory.path().join("zxs-data").exists());
}

#[test]
fn rescue_restores_from_two_old_members_without_management_commands() {
    use storage_server::{auth::AuthStore, engine_backend::EngineBackend};
    let directory = tempfile::tempdir().unwrap();
    let original = directory.path().join("original");
    let replacement = directory.path().join("replacement");
    let disks = ["a", "b", "c"].map(|slot| directory.path().join(slot));
    let mut engine = storage_engine::Engine::initialize(
        &original,
        storage_engine::Config::development(disks.clone()),
    )
    .unwrap();
    let repository = engine
        .create_repository(
            "rescue-fixture",
            "P01",
            3,
            "admin",
            &storage_engine::new_id("op"),
        )
        .unwrap();
    let backend = std::sync::Arc::new(EngineBackend::new(engine));
    backend
        .protect_auth(AuthStore::initialize(&original, "admin", "test-only-password").unwrap())
        .unwrap();
    drop(backend);
    std::fs::remove_dir_all(&original).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_zxs-rescue"))
        .arg("--root")
        .arg(&replacement)
        .args(["restore-system", "--disks"])
        .args(&disks[..2])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let receipt: json_codec::Value = json_codec::from_slice(&output.stdout).unwrap();
    assert_eq!(receipt["credential_recovery_snapshot_available"], true);
    let restored = storage_engine::Engine::open(&replacement).unwrap();
    assert!(
        restored
            .state()
            .repositories
            .contains_key(&repository.repository_id)
    );
    assert!(restored.auth_state().is_some());
}

#[test]
fn history_package_round_trip_preserves_checks_and_refuses_overwrite() {
    use std::os::unix::fs::PermissionsExt;
    use storage_engine::{CommitRequest, Config, Engine, new_id};
    let directory = tempfile::tempdir().unwrap();
    std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let source = directory.path().join("source");
    let target = directory.path().join("target");
    let initialize = |root: &std::path::Path, prefix: &str| {
        let disks = ["a", "b", "c"].map(|slot| directory.path().join(format!("{prefix}-{slot}")));
        Engine::initialize(root, Config::development(disks)).unwrap()
    };
    let mut engine = initialize(&source, "source");
    let repository = engine
        .create_repository("package-fixture", "P01", 3, "admin", &new_id("op"))
        .unwrap();
    let workspace = engine
        .create_workspace(&repository.main_branch_id, "admin")
        .unwrap();
    engine
        .stage_bytes(
            &workspace.workspace_id,
            "verified.txt",
            b"verified package bytes",
        )
        .unwrap();
    engine
        .commit_workspace(CommitRequest {
            workspace_id: workspace.workspace_id,
            operation_id: new_id("op"),
            principal: "admin".into(),
            expected_head: workspace.base_head,
            fencing_token: workspace.fencing_token,
            message: "package fixture".into(),
            request_digest: None,
        })
        .unwrap();
    drop(engine);
    let package = directory.path().join("package");
    let invoke = |root: &std::path::Path, args: &[&std::ffi::OsStr]| {
        Command::new(env!("CARGO_BIN_EXE_zxs-rescue"))
            .arg("--root")
            .arg(root)
            .arg("package")
            .args(args)
            .output()
            .unwrap()
    };
    let export = [
        "export".as_ref(),
        repository.history_tree_id.as_ref(),
        package.as_os_str(),
    ];
    let output = invoke(&source, &export);
    assert!(output.status.success(), "{output:?}");
    assert!(
        !invoke(&source, &export).status.success(),
        "must never overwrite package destination"
    );
    assert!(
        invoke(&source, &["check".as_ref(), package.as_os_str()])
            .status
            .success()
    );
    drop(initialize(&target, "target"));
    let output = invoke(
        &target,
        &[
            "restore".as_ref(),
            package.as_os_str(),
            "--primary-slot".as_ref(),
            "P01".as_ref(),
        ],
    );
    assert!(output.status.success(), "{output:?}");
    let restored = Engine::open(&target).unwrap();
    assert!(
        restored
            .state()
            .repositories
            .contains_key(&repository.repository_id)
    );
    let entries = restored.list_tree(&repository.main_branch_id).unwrap();
    assert_eq!(entries.len(), 1);
    let mut content = restored.open_content(&entries[0].version_id).unwrap();
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut content.file, &mut bytes).unwrap();
    assert_eq!(bytes, b"verified package bytes");
    drop(content);
    drop(restored);
    std::fs::write(package.join("publication.json"), b"invalid proof").unwrap();
    assert!(
        !invoke(&source, &["check".as_ref(), package.as_os_str()])
            .status
            .success()
    );
}

#[test]
fn configuration_commands_edit_only_an_uninitialized_draft() {
    let temporary = tempfile::tempdir().unwrap();
    let directory = temporary.path();
    let output = directory.join("custom.json");
    let disks = [
        directory.join("p"),
        directory.join("b"),
        directory.join("c"),
    ];
    let binary = env!("CARGO_BIN_EXE_zxs-rescue");
    let result = Command::new(binary)
        .args(["configuration", "template", "--disks"])
        .args(&disks)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let original = std::fs::read(&output).unwrap();
    let additional = directory.join("fourth");
    let result = Command::new(binary)
        .args(["configuration", "register-disk", "--config"])
        .arg(&output)
        .args(["--slot", "P02", "--disk-id"])
        .arg(storage_engine::new_id("disk"))
        .arg("--path")
        .arg(&additional)
        .args(["--level", "1"])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let config: storage_engine::Config =
        storage_engine::decode_strict(&std::fs::read(&output).unwrap()).unwrap();
    assert_eq!(config.disks.len(), 4);
    assert_eq!(
        std::fs::read(directory.join("custom.json.before-registration")).unwrap(),
        original
    );
    assert!(config.disks.iter().all(|disk| !disk.root.exists()));
    assert!(!directory.join("auth.json").exists());
    let result = Command::new(binary)
        .args(["configuration", "template", "--disks"])
        .args(&disks)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!result.status.success());
}
