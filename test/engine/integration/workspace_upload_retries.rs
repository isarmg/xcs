//! Publication can succeed before either workspace manifest is durable.
use std::{fs, os::unix::fs::PermissionsExt, path::Path};
use storage_engine::{Config, Engine, new_id};

fn corrupt_retained_bytes(path: &Path) {
    // Published generations are read-only. Deliberate out-of-band corruption
    // must not rely on the test runner having root's permission bypass.
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(path, b"oops").unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o400)).unwrap();
}

#[test]
fn upload_retry_recovers_published_bytes_after_manifest_failure() {
    for primary_failure in [true, false] {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("system");
        let primary = temporary.path().join("primary");
        let mut config = Config::development([
            primary.clone(),
            temporary.path().join("backup"),
            temporary.path().join("tertiary"),
        ]);
        config.max_file_bytes = 4;
        config.max_workspace_bytes = 4;
        let mut engine = Engine::initialize(&root, config).unwrap();
        let repo = engine
            .create_repository("Retry uploads", "P01", 1, "admin", &new_id("op"))
            .unwrap();
        let workspace = engine
            .create_workspace(&repo.main_branch_id, "admin")
            .unwrap();
        let item = new_id("entry");
        let source = temporary.path().join("source");
        fs::write(&source, b"data").unwrap();
        let data = primary
            .join("staging/workspaces")
            .join(&workspace.workspace_id);
        let manifest = if primary_failure {
            data.join("workspace.json")
        } else {
            root.join("workspaces")
                .join(&workspace.workspace_id)
                .join("workspace.json")
        };
        let original_manifest = fs::read(&manifest).unwrap();
        fs::remove_file(&manifest).unwrap();
        fs::create_dir(&manifest).unwrap();
        assert!(
            engine
                .stage_upload(&workspace.workspace_id, &item, "a", &source, 1)
                .is_err()
        );
        let published = data.join(format!("{item}-1.full"));
        assert_eq!(fs::read(&published).unwrap(), b"data");
        fs::remove_dir(&manifest).unwrap();
        fs::write(&manifest, original_manifest).unwrap();
        // A possibly durable mirror write fences this process even if
        // the failing path was repaired externally.
        assert_eq!(
            engine
                .stage_upload(&workspace.workspace_id, &item, "a", &source, 1)
                .unwrap_err()
                .code(),
            "RECOVERY_REQUIRED"
        );
        assert_eq!(fs::read(&published).unwrap(), b"data");
        drop(engine);
        if !primary_failure {
            // The primary advanced, but the restored system manifest is
            // valid-old. No publication receipt proves an upload-only
            // transition, so neither valid mirror may silently win.
            let data_manifest = data.join("workspace.json");
            let retained_data = fs::read(&data_manifest).unwrap();
            let retained_system = fs::read(&manifest).unwrap();
            assert_eq!(
                Engine::open(&root).err().unwrap().code(),
                "RECOVERY_REQUIRED"
            );
            assert_eq!(fs::read(data_manifest).unwrap(), retained_data);
            assert_eq!(fs::read(manifest).unwrap(), retained_system);
            assert_eq!(fs::read(&published).unwrap(), b"data");
            continue;
        }
        engine = Engine::open(&root).unwrap();
        // A retry must never overwrite an already published generation.
        fs::write(&source, b"oops").unwrap();
        assert_eq!(
            engine
                .stage_upload(&workspace.workspace_id, &item, "a", &source, 1)
                .unwrap_err()
                .code(),
            "UPLOAD_GENERATION_CONFLICT"
        );
        assert_eq!(fs::read(&published).unwrap(), b"data");
        fs::write(&source, b"data").unwrap();
        let staged = engine
            .stage_upload(&workspace.workspace_id, &item, "a", &source, 1)
            .unwrap();
        assert_eq!(staged.source_path, published);
        assert_eq!(staged.size, 4);
        assert_eq!(
            engine.workspace(&workspace.workspace_id).unwrap().files[&item].content_hash,
            staged.content_hash
        );
        assert_eq!(
            engine
                .stage_upload(&workspace.workspace_id, &new_id("entry"), "b", &source, 1)
                .unwrap_err()
                .code(),
            "RESOURCE_LIMIT"
        );
        drop(engine);
        let engine = Engine::open(&root).unwrap();
        assert_eq!(
            engine.workspace(&workspace.workspace_id).unwrap().files[&item].content_hash,
            staged.content_hash
        );
    }
}

#[test]
fn same_generation_retry_rejects_missing_or_corrupt_retained_bytes() {
    for missing in [false, true] {
        for reopen in [false, true] {
            let temporary = tempfile::tempdir().unwrap();
            let root = temporary.path().join("system");
            let mut engine = Engine::initialize(
                &root,
                Config::development([
                    temporary.path().join("primary"),
                    temporary.path().join("backup"),
                    temporary.path().join("tertiary"),
                ]),
            )
            .unwrap();
            let repo = engine
                .create_repository("Retained upload", "P01", 1, "admin", &new_id("op"))
                .unwrap();
            let workspace = engine
                .create_workspace(&repo.main_branch_id, "admin")
                .unwrap();
            let item = new_id("entry");
            let source = temporary.path().join("source");
            fs::write(&source, b"data").unwrap();
            let staged = engine
                .stage_upload(&workspace.workspace_id, &item, "a", &source, 1)
                .unwrap();
            if missing {
                fs::remove_file(&staged.source_path).unwrap();
            } else {
                corrupt_retained_bytes(&staged.source_path);
            }
            if reopen {
                drop(engine);
                engine = Engine::open(&root).unwrap();
            }
            assert!(
                engine
                    .stage_upload(&workspace.workspace_id, &item, "a", &source, 1)
                    .is_err()
            );
            let retained = &engine.workspace(&workspace.workspace_id).unwrap().files[&item];
            assert_eq!(retained.content_hash, staged.content_hash);
            assert_eq!(retained.dirty_generation, 1);
            if missing {
                assert!(!staged.source_path.exists());
            } else {
                assert_eq!(fs::read(&staged.source_path).unwrap(), b"oops");
            }
        }
    }
}

#[test]
fn same_generation_retry_revalidates_held_target_before_acknowledging() {
    for replace in [false, true] {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("system");
        let mut engine = Engine::initialize(
            &root,
            Config::development([
                temporary.path().join("primary"),
                temporary.path().join("backup"),
                temporary.path().join("tertiary"),
            ]),
        )
        .unwrap();
        let repo = engine
            .create_repository("Retained upload", "P01", 1, "admin", &new_id("op"))
            .unwrap();
        let workspace = engine
            .create_workspace(&repo.main_branch_id, "admin")
            .unwrap();
        let item = new_id("entry");
        let source = temporary.path().join("source");
        fs::write(&source, b"data").unwrap();
        let staged = engine
            .stage_upload(&workspace.workspace_id, &item, "a", &source, 1)
            .unwrap();
        let prepared = engine
            .prepare_upload(&workspace.workspace_id, &item, "a", &source, 1)
            .unwrap()
            .execute()
            .unwrap();
        if replace {
            fs::rename(
                &staged.source_path,
                temporary.path().join("held-generation"),
            )
            .unwrap();
            // Even equal replacement bytes are not the inode already verified.
            fs::write(&staged.source_path, b"data").unwrap();
        } else {
            corrupt_retained_bytes(&staged.source_path);
        }
        assert_eq!(
            engine.finish_upload(prepared).unwrap_err().code(),
            "CHECKSUM_MISMATCH"
        );
        assert_eq!(
            engine.workspace(&workspace.workspace_id).unwrap().files[&item].content_hash,
            staged.content_hash
        );
    }
}
