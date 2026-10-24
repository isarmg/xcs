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

fn tombstoned_body(state: &str) -> (tempfile::TempDir, Engine, Config, FileVersion) {
    let temporary = tempfile::tempdir().unwrap();
    fs::set_permissions(temporary.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let root = temporary.path().join("source");
    let config = Config::development([
        root.join("primary"),
        root.join("backup"),
        root.join("third"),
    ]);
    let mut engine = Engine::initialize(&root.join("system"), config.clone()).unwrap();
    let repository = engine
        .create_repository("Tombstones", "P01", 3, "owner", &new_id("op"))
        .unwrap();
    let mut saved = Vec::new();
    for bytes in [b"old", b"new"] {
        let workspace = engine
            .create_workspace(&repository.main_branch_id, "owner")
            .unwrap();
        engine
            .stage_bytes(&workspace.workspace_id, "body.txt", bytes)
            .unwrap();
        let committed = engine
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
        saved.push((
            committed.commit_id.unwrap(),
            engine.list_tree(&repository.main_branch_id).unwrap()[0]
                .version_id
                .clone(),
        ));
    }
    let (cut, old) = &saved[0];
    let plan = match state {
        "PURGED" => engine
            .create_purge_plan(&repository.history_tree_id, vec![old.clone()], "owner")
            .unwrap(),
        "MIGRATED" => engine
            .create_split_plan(&repository.history_tree_id, cut, "owner")
            .unwrap(),
        _ => panic!("unexpected tombstone state"),
    };
    engine
        .commit_plan(&plan.plan_id, "owner", &new_id("op"))
        .unwrap();
    assert_eq!(engine.state().content_states[old], state);
    let version = engine.state().versions[old].clone();
    (temporary, engine, config, version)
}

#[test]
fn offline_tombstones_bind_retained_version_metadata_and_directory() {
    for state in ["PURGED", "MIGRATED"] {
        let (temporary, engine, config, version) = tombstoned_body(state);
        drop(engine);
        let disk = &config.disks[0];
        let directory = version_dir(disk, &version.history_tree_id, &version.version_id);
        let baseline = offline_check(&disk.root).unwrap();
        assert!(baseline.issues.is_empty(), "{state}: {:?}", baseline.issues);
        assert!(baseline.tombstoned_versions > 0);
        let destination = temporary.path().join("denied.txt");
        assert_eq!(
            offline_export(&disk.root, &version.version_id, &destination)
                .unwrap_err()
                .code(),
            "CONTENT_PURGED"
        );
        assert!(!destination.exists());

        let assert_corrupt = |case| {
            let report = offline_check(&disk.root).unwrap();
            assert_eq!(
                report.issues.len(),
                1,
                "{state}/{case}: {:?}",
                report.issues
            );
            assert_eq!(report.tombstoned_versions + 1, baseline.tombstoned_versions);
            assert_eq!(report.healthy_replicas, baseline.healthy_replicas);
        };
        for field in ["content_hash", "history_tree_id"] {
            let mut damaged = version.clone();
            match field {
                "content_hash" => damaged.content_hash = "0".repeat(64),
                "history_tree_id" => damaged.history_tree_id = new_id("history"),
                _ => unreachable!(),
            }
            fs::write(
                directory.join("version.json"),
                canonical_json(&damaged).unwrap(),
            )
            .unwrap();
            assert_corrupt(field);
        }
        fs::write(
            directory.join("version.json"),
            canonical_json(&version).unwrap(),
        )
        .unwrap();

        let moved = directory.with_file_name(new_id("ver"));
        fs::rename(&directory, &moved).unwrap();
        assert_corrupt("directory");
        fs::rename(&moved, &directory).unwrap();

        let proof = directory.join("publication-proof.json");
        let saved = fs::read(&proof).unwrap();
        fs::remove_file(&proof).unwrap();
        assert_corrupt("missing local publication");
        fs::write(proof, saved).unwrap();
        assert!(offline_check(&disk.root).unwrap().issues.is_empty());
    }
}

#[test]
fn restored_tombstones_need_no_source_payload_proofs() {
    for state in ["PURGED", "MIGRATED"] {
        let (temporary, engine, _config, version) = tombstoned_body(state);
        let package = temporary.path().join("package");
        let exported = engine
            .export_history_package(&version.history_tree_id, &package)
            .unwrap();
        assert!(exported.tombstones > 0);
        drop(engine);
        fs::remove_dir_all(temporary.path().join("source")).unwrap();
        let root = temporary.path().join("restored");
        let config = Config::development([
            root.join("primary"),
            root.join("backup"),
            root.join("third"),
        ]);
        let mut restored = Engine::initialize(&root.join("system"), config.clone()).unwrap();
        restored
            .restore_history_package(&package, "P01", 3, "owner", &new_id("op"))
            .unwrap();
        assert_eq!(restored.state().content_states[&version.version_id], state);
        drop(restored);
        for disk in &config.disks {
            // Imported retained identities have no local payload to purge and
            // therefore do not fabricate an original payload publication proof.
            assert!(!version_dir(disk, &version.history_tree_id, &version.version_id).exists());
            assert!(offline_check(&disk.root).unwrap().issues.is_empty());
        }
    }
}
