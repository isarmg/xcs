//! Offline draft edits must never rewrite a live pool or publish invalid topology.
use super::*;
use std::{fs, path::PathBuf};

fn draft(root: &Path) -> Config {
    Config::development([root.join("p"), root.join("b"), root.join("c")])
}

fn registration(root: &Path, config: PathBuf) -> Args {
    Args {
        root: root.join("unrelated-system"),
        principal: "admin".into(),
        server: None,
        token_file: None,
        command: Command::RegisterDisk {
            config,
            slot: "P02".into(),
            disk_id: storage_engine::new_id("disk"),
            path: root.join("additional"),
            level: 1,
            upstream: None,
            metadata_member: false,
        },
    }
}

#[tokio::test]
async fn unrelated_root_cannot_edit_a_live_configuration_or_its_aliases() {
    use std::os::unix::fs::symlink;

    for kind in ["direct", "parent_alias", "file_alias", "hard_link", "copy"] {
        let root = tempfile::tempdir().unwrap();
        let system = root.path().join("system");
        let engine = Engine::initialize(&system, draft(root.path())).unwrap();
        // No auth.json is needed to make an initialized pool configuration live.
        assert!(!system.join("auth.json").exists());
        let live = system.join("config.json");
        let original = fs::read(&live).unwrap();
        let selected = match kind {
            "parent_alias" => {
                let alias = root.path().join("system-alias");
                symlink(&system, &alias).unwrap();
                alias.join("config.json")
            }
            "file_alias" => {
                let alias = root.path().join("draft-alias.json");
                symlink(&live, &alias).unwrap();
                alias
            }
            "hard_link" => {
                let alias = root.path().join("draft-link.json");
                fs::hard_link(&live, &alias).unwrap();
                alias
            }
            "copy" => {
                let copy = root.path().join("copied-draft.json");
                private_io::write_new(&copy, &original).unwrap();
                copy
            }
            _ => live.clone(),
        };
        assert!(
            run(registration(root.path(), selected.clone()))
                .await
                .is_err(),
            "{kind} must not modify an initialized pool through an unrelated --root"
        );
        assert_eq!(fs::read(&live).unwrap(), original, "{kind}");
        assert_eq!(fs::read(&selected).unwrap(), original, "{kind}");
        assert!(!registration_backup(&selected).unwrap().exists(), "{kind}");
        drop(engine);
    }
}

#[tokio::test]
async fn invalid_registration_candidates_preserve_the_draft_and_backup_namespace() {
    for kind in [
        "slot",
        "slot_case",
        "disk_id",
        "relative",
        "overlap",
        "membership",
        "forked_route",
        "resource_limit",
        "level_overflow",
    ] {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("draft.json");
        let mut config = draft(root.path());
        let mut args = registration(root.path(), path.clone());
        let Command::RegisterDisk {
            slot,
            disk_id,
            path: disk,
            level,
            upstream,
            metadata_member,
            ..
        } = &mut args.command
        else {
            unreachable!()
        };
        match kind {
            "slot" => *slot = "bad/slot".into(),
            "slot_case" => *slot = "p01".into(),
            "disk_id" => *disk_id = "not-a-disk-id".into(),
            "relative" => *disk = PathBuf::from("relative-root"),
            "overlap" => *disk = config.disks[0].root.join("nested"),
            "membership" => *metadata_member = true,
            "forked_route" => {
                *level = 2;
                *upstream = Some("P01".into());
            }
            "resource_limit" => config.max_file_bytes = 0,
            "level_overflow" => config.disks[0].level = u32::MAX,
            _ => unreachable!(),
        }
        let original = serde_json::to_vec(&config).unwrap();
        private_io::write_new(&path, &original).unwrap();
        assert!(
            run(args).await.is_err(),
            "{kind} must be rejected before any write"
        );
        assert_eq!(fs::read(&path).unwrap(), original, "{kind}");
        assert!(!registration_backup(&path).unwrap().exists(), "{kind}");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1, "{kind}");
    }
}

#[tokio::test]
async fn newly_added_disk_cannot_already_belong_to_another_pool() {
    let root = tempfile::tempdir().unwrap();
    let other = root.path().join("other-pool");
    fs::create_dir(&other).unwrap();
    let engine = Engine::initialize(&other.join("system"), draft(&other)).unwrap();
    let config = draft(root.path());
    let path = root.path().join("draft.json");
    let original = serde_json::to_vec(&config).unwrap();
    private_io::write_new(&path, &original).unwrap();
    let mut args = registration(root.path(), path.clone());
    if let Command::RegisterDisk { path, .. } = &mut args.command {
        *path = engine.config().disks[0].root.clone();
    }
    assert!(run(args).await.is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
    assert!(!registration_backup(&path).unwrap().exists());
}

#[tokio::test]
async fn aliases_overlapping_roots_and_existing_markers_preserve_a_previous_backup() {
    use std::os::unix::fs::symlink;

    for kind in [
        "alias",
        "missing_alias_child",
        "dangling_link",
        "parent_traversal",
        "system_marker",
        "seal",
        "nested_seal",
        "membership",
        "file_root",
    ] {
        let root = tempfile::tempdir().unwrap();
        let config = draft(root.path());
        let path = root.path().join("draft.json");
        let original = serde_json::to_vec(&config).unwrap();
        private_io::write_new(&path, &original).unwrap();
        let backup = registration_backup(&path).unwrap();
        private_io::write_new(&backup, b"earlier recovery draft").unwrap();
        let mut args = registration(root.path(), path.clone());
        let Command::RegisterDisk { path: new_disk, .. } = &mut args.command else {
            unreachable!()
        };
        match kind {
            "alias" | "missing_alias_child" => {
                fs::create_dir(&config.disks[0].root).unwrap();
                symlink(&config.disks[0].root, &*new_disk).unwrap();
                if kind == "missing_alias_child" {
                    *new_disk = new_disk.join("not-created");
                }
            }
            "dangling_link" => symlink(root.path().join("absent"), &*new_disk).unwrap(),
            "parent_traversal" => *new_disk = root.path().join("p/../b"),
            "system_marker" => fs::write(root.path().join("writer.lock"), b"").unwrap(),
            "seal" | "nested_seal" => {
                fs::create_dir(&*new_disk).unwrap();
                fs::write(new_disk.join("disk.json"), b"managed identity").unwrap();
                if kind == "nested_seal" {
                    *new_disk = new_disk.join("not-created");
                }
            }
            "membership" => {
                fs::create_dir_all(new_disk.join("metadata/pool-membership")).unwrap();
                fs::write(
                    new_disk.join("metadata/pool-membership/config.json"),
                    b"pool",
                )
                .unwrap();
            }
            "file_root" => fs::write(&*new_disk, b"not a directory").unwrap(),
            _ => unreachable!(),
        }
        assert!(run(args).await.is_err(), "{kind}");
        assert_eq!(fs::read(&path).unwrap(), original, "{kind}");
        assert_eq!(
            fs::read(&backup).unwrap(),
            b"earlier recovery draft",
            "{kind}"
        );
    }
}

#[tokio::test]
async fn ordinary_draft_permissions_remain_supported_without_initializing_disks() {
    use std::os::unix::fs::PermissionsExt;

    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("draft.json");
    let original = serde_json::to_vec(&draft(root.path())).unwrap();
    fs::write(&path, &original).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    run(registration(root.path(), path.clone())).await.unwrap();
    let saved: Config = storage_engine::decode_strict(&fs::read(&path).unwrap()).unwrap();
    Engine::validate_configuration(&saved).unwrap();
    assert!(saved.disks.iter().all(|disk| !disk.root.exists()));
    assert_eq!(
        fs::read(registration_backup(&path).unwrap()).unwrap(),
        original
    );
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
}
