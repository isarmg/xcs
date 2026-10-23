use super::*;

fn published_body() -> (tempfile::TempDir, PathBuf, FileVersion) {
    let temporary = tempfile::tempdir().unwrap();
    let config = Config::development([
        temporary.path().join("primary"),
        temporary.path().join("backup"),
        temporary.path().join("third"),
    ]);
    let mut engine = Engine::initialize(&temporary.path().join("system"), config.clone()).unwrap();
    let repository = engine
        .create_repository("Export", "P01", 3, "owner", &new_id("op"))
        .unwrap();
    let workspace = engine
        .create_workspace(&repository.main_branch_id, "owner")
        .unwrap();
    engine
        .stage_bytes(&workspace.workspace_id, "body.txt", b"published bytes")
        .unwrap();
    engine
        .commit_workspace(CommitRequest {
            workspace_id: workspace.workspace_id,
            operation_id: new_id("op"),
            principal: "owner".into(),
            expected_head: workspace.base_head,
            fencing_token: workspace.fencing_token,
            message: String::new(),
            request_digest: None,
        })
        .unwrap();
    let entry = engine
        .list_tree(&repository.main_branch_id)
        .unwrap()
        .remove(0);
    let version = engine.state().versions[&entry.version_id].clone();
    let directory = version_dir(
        &config.disks[0],
        &version.history_tree_id,
        &version.version_id,
    );
    drop(engine);
    (temporary, directory, version)
}

#[test]
fn verified_export_rejects_truncated_and_changed_bytes_before_publication() {
    for bytes in [b"".as_slice(), b"different bytes".as_slice()] {
        let (temporary, directory, version) = published_body();
        let source = verify_payload_held(&directory, &version).unwrap();
        let path = crate::container_storage::content_path(&directory, &version).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        fs::write(path, bytes).unwrap();
        let destination = temporary.path().join("exports").join("body.txt");
        assert_eq!(
            verified_full_copy(source, &destination, &version)
                .unwrap_err()
                .code(),
            "SOURCE_CHANGED"
        );
        assert!(
            !destination.exists(),
            "invalid bytes must never reach the final name"
        );
        assert_eq!(
            fs::read_dir(destination.parent().unwrap()).unwrap().count(),
            0,
            "temporary output must be cleaned up"
        );
    }
}

#[test]
fn verified_export_pins_checked_inode_and_keeps_no_overwrite_policy() {
    let (temporary, directory, version) = published_body();
    let source = verify_payload_held(&directory, &version).unwrap();
    let path = crate::container_storage::content_path(&directory, &version).unwrap();
    fs::rename(&path, path.with_extension("old")).unwrap();
    fs::write(&path, b"replacement data").unwrap();
    let destination = temporary.path().join("exports").join("body.txt");
    verified_full_copy(source, &destination, &version).unwrap();
    assert_eq!(fs::read(&destination).unwrap(), b"published bytes");
    let checked = open_regular(&path.with_extension("old")).unwrap();
    assert!(verified_full_copy(checked, &destination, &version).is_err());
    assert_eq!(fs::read(&destination).unwrap(), b"published bytes");
    assert_eq!(
        fs::read_dir(destination.parent().unwrap()).unwrap().count(),
        1
    );
}

#[test]
fn verified_export_compares_checksum_commitment_before_publication() {
    let (temporary, directory, mut version) = published_body();
    let source = verify_payload_held(&directory, &version).unwrap();
    version.checksum_manifest_hash = "0".repeat(64);
    let destination = temporary.path().join("exports").join("body.txt");
    assert_eq!(
        verified_full_copy(source, &destination, &version)
            .unwrap_err()
            .code(),
        "SOURCE_CHANGED"
    );
    assert!(!destination.exists());
    assert_eq!(
        fs::read_dir(destination.parent().unwrap()).unwrap().count(),
        0
    );
}
