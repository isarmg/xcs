//! Publication can succeed before either workspace manifest is durable.
use std::fs;
use xcs_core::{Config, Engine, new_id};

#[test]
fn upload_retry_recovers_published_bytes_after_manifest_failure() {
    for primary_failure in [true, false] {
        for restart in [false, true] {
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
            if restart {
                drop(engine);
                engine = Engine::open(&root).unwrap();
            }
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
}
