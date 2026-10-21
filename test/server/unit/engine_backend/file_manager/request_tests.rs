//! Malformed requests must be rejected before lazy disk-root publication.
use super::*;
use crate::BackendAction;
use std::path::{Path, PathBuf};

fn disk_files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(path: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                visit(&entry.path(), files);
            } else {
                files.insert(entry.path(), std::fs::read(entry.path()).unwrap());
            }
        }
    }
    let mut files = BTreeMap::new();
    visit(root, &mut files);
    files
}

#[test]
fn explicit_non_string_paths_never_mean_the_root() {
    for value in [Value::Null, json!(123), json!([]), json!({}), json!(false)] {
        assert!(path(&json!({"path":value})).is_err(), "{value}");
        assert!(path(&json!({"query":{"path":value}})).is_err(), "{value}");
        assert!(path(&json!({"path":value,"query":{"path":"valid"}})).is_err());
    }
    assert_eq!(path(&json!({})).unwrap(), "");
    assert_eq!(path(&json!({"path":""})).unwrap(), "");
    assert_eq!(path(&json!({"query":{"path":"folder"}})).unwrap(), "folder");
}

#[test]
fn malformed_lazy_root_requests_do_not_publish_any_state() {
    use BackendAction::*;
    let op = storage_engine::new_id("op");
    let cases = [
        (
            FileManagerDirectoryBranches,
            json!({"slot_id":"P01","path":"","operation_id":op}),
        ),
        (
            FileManagerDirectoryBranches,
            json!({"slot_id":"P01","path":"","name":"branch","operation_id":"bad"}),
        ),
        (
            FileManagerDirectoryBranches,
            json!({"slot_id":"P01","path":"","name":"x".repeat(129),"operation_id":op}),
        ),
        (
            FileManagerDirectorySelect,
            json!({"slot_id":"P01","path":"","name":null,"operation_id":op}),
        ),
        (
            FileManagerWriteSession,
            json!({"slot_id":"P01","path":"file.txt","branch_id":123}),
        ),
        (
            FileManagerNamespace,
            json!({"slot_id":"P01","path":"folder","kind":"unknown","operation_id":op}),
        ),
        (
            FileManagerNamespace,
            json!({"slot_id":"P01","path":"folder","kind":"mkdir"}),
        ),
        (
            FileManagerNamespace,
            json!({"slot_id":"P01","path":"folder","kind":"mkdir","operation_id":"bad"}),
        ),
        (
            FileManagerNamespace,
            json!({"slot_id":"P01","path":"folder","kind":"rename","destination":123,"operation_id":op}),
        ),
        (
            FileManagerNamespace,
            json!({"slot_id":"P01","path":"folder","kind":"rename","operation_id":op}),
        ),
        (
            FileManagerNamespace,
            json!({"slot_id":"P01","path":"folder","kind":"rename","destination":"../escape","operation_id":op}),
        ),
    ];
    for (action, input) in cases {
        let root = tempfile::tempdir().unwrap();
        let disks = root.path().join("disks");
        let config = storage_engine::Config::development([
            disks.join("one"),
            disks.join("two"),
            disks.join("three"),
        ]);
        let mut engine = Engine::initialize(&root.path().join("system"), config).unwrap();
        let identity = Identity {
            principal: "owner".into(),
            admin: true,
            repositories: BTreeSet::new(),
        };
        let before = serde_json::to_value(engine.state()).unwrap();
        let bytes = disk_files(&disks);
        let failure = call(&mut engine, &identity, action, input.clone()).unwrap_err();
        assert_eq!(failure.status, 400, "{input}: {failure:?}");
        assert_eq!(
            serde_json::to_value(engine.state()).unwrap(),
            before,
            "{input}"
        );
        assert_eq!(disk_files(&disks), bytes, "{input}");
    }
}

#[test]
fn missing_targets_and_cross_namespace_moves_do_not_create_a_disk_root() {
    let root = tempfile::tempdir().unwrap();
    let disks = root.path().join("disks");
    let mut engine = Engine::initialize(
        &root.path().join("system"),
        storage_engine::Config::development([
            disks.join("one"),
            disks.join("two"),
            disks.join("three"),
        ]),
    )
    .unwrap();
    let identity = Identity {
        principal: "owner".into(),
        admin: true,
        repositories: BTreeSet::new(),
    };
    for kind in ["delete", "delete_tree", "rename"] {
        let mut input = json!({"slot_id":"P01","kind":kind,"path":"missing","operation_id":storage_engine::new_id("op")});
        if kind == "rename" {
            input["destination"] = json!("destination");
        }
        assert_eq!(
            call(
                &mut engine,
                &identity,
                BackendAction::FileManagerNamespace,
                input
            )
            .unwrap_err()
            .status,
            404
        );
        assert!(engine.state().repositories.is_empty());
        assert_eq!(engine.state().sequence, 0);
    }
    let repo = engine
        .create_repository("Existing", "P01", 3, "owner", &storage_engine::new_id("op"))
        .unwrap();
    engine
        .file_namespace(
            &repo.repository_id,
            NamespaceOperation {
                kind: "mkdir".into(),
                path: "folder".into(),
                destination: None,
            },
            "owner",
            &storage_engine::new_id("op"),
        )
        .unwrap();
    let before = serde_json::to_value(engine.state()).unwrap();
    let bytes = disk_files(&disks);
    assert!(call(&mut engine, &identity, BackendAction::FileManagerNamespace, json!({"slot_id":"P01","kind":"rename","path":"Existing/folder","destination":"new-root-folder","operation_id":storage_engine::new_id("op")})).is_err());
    assert_eq!(serde_json::to_value(engine.state()).unwrap(), before);
    assert_eq!(disk_files(&disks), bytes);
}

#[test]
fn valid_creation_and_upload_still_create_the_lazy_root() {
    for action in [
        BackendAction::FileManagerNamespace,
        BackendAction::FileManagerDirectoryBranches,
        BackendAction::FileManagerWriteSession,
    ] {
        let root = tempfile::tempdir().unwrap();
        let mut engine = Engine::initialize(
            &root.path().join("system"),
            storage_engine::Config::development([
                root.path().join("one"),
                root.path().join("two"),
                root.path().join("three"),
            ]),
        )
        .unwrap();
        let identity = Identity {
            principal: "owner".into(),
            admin: true,
            repositories: BTreeSet::new(),
        };
        let input = match action {
            BackendAction::FileManagerNamespace => {
                json!({"slot_id":"P01","kind":"mkdir","path":"folder","operation_id":storage_engine::new_id("op")})
            }
            BackendAction::FileManagerDirectoryBranches => {
                json!({"slot_id":"P01","path":"","name":"branch","operation_id":storage_engine::new_id("op")})
            }
            _ => json!({"slot_id":"P01","path":"file.txt"}),
        };
        call(&mut engine, &identity, action, input).unwrap();
        assert_eq!(engine.state().repositories.len(), 1);
        assert!(
            engine
                .state()
                .repositories
                .values()
                .next()
                .unwrap()
                .file_manager_root
        );
    }
}

#[test]
fn baseline_name_and_used_operation_are_rejected_before_root_initialization() {
    let root = tempfile::tempdir().unwrap();
    let disks = root.path().join("disks");
    let mut engine = Engine::initialize(
        &root.path().join("system"),
        storage_engine::Config::development([
            disks.join("one"),
            disks.join("two"),
            disks.join("three"),
        ]),
    )
    .unwrap();
    let identity = Identity {
        principal: "owner".into(),
        admin: true,
        repositories: BTreeSet::new(),
    };
    for name in ["main", "MAIN"] {
        let before = disk_files(&disks);
        let error = call(&mut engine, &identity, BackendAction::FileManagerDirectoryBranches, json!({"slot_id":"P01","path":"","name":name,"operation_id":storage_engine::new_id("op")})).unwrap_err();
        assert_eq!(error.code, "NAME_COLLISION");
        assert_eq!(engine.state().sequence, 0);
        assert_eq!(disk_files(&disks), before);
    }
    let used = storage_engine::new_id("op");
    engine
        .create_repository("Existing", "P01", 3, "owner", &used)
        .unwrap();
    let before = serde_json::to_value(engine.state()).unwrap();
    let bytes = disk_files(&disks);
    let error = call(
        &mut engine,
        &identity,
        BackendAction::FileManagerNamespace,
        json!({"slot_id":"P01","path":"fresh","kind":"mkdir","operation_id":used}),
    )
    .unwrap_err();
    assert_eq!(error.code, "IDEMPOTENCY_KEY_REUSED");
    assert_eq!(serde_json::to_value(engine.state()).unwrap(), before);
    assert_eq!(disk_files(&disks), bytes);
}
