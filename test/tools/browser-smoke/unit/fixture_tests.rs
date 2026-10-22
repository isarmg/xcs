use super::*;
use storage_engine::Config;

fn pool() -> tempfile::TempDir {
    let temporary = tempfile::tempdir().unwrap();
    let disks = ["P01", "B01", "C01"].map(|slot| temporary.path().join(slot));
    drop(Engine::initialize(&temporary.path().join("pool"), Config::development(disks)).unwrap());
    temporary
}

#[test]
fn test_only_fixture_persists_two_versions_and_refuses_reuse() {
    let temporary = pool();
    seed_files(&temporary).unwrap();
    let engine = Engine::open(&temporary.path().join("pool")).unwrap();
    let repository = engine.state().repositories.values().next().unwrap();
    assert_eq!(repository.name, "browser-fixtures");
    let entries = engine.list_tree(&repository.main_branch_id).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].path, FILE);
    let versions = engine.file_history(&entries[0].file_id).unwrap();
    assert_eq!(versions.len(), 2);
    for (version, expected) in versions.iter().zip(PAYLOADS) {
        let mut content = engine.open_content(&version.version_id).unwrap();
        let mut bytes = Vec::new();
        content.file.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, expected.as_bytes());
    }
    drop(engine);
    let error = seed_files(&temporary).unwrap_err();
    assert!(error.to_string().contains("fresh disposable pool"));
    let engine = Engine::open(&temporary.path().join("pool")).unwrap();
    assert_eq!(engine.file_history(&entries[0].file_id).unwrap().len(), 2);
}

#[test]
fn fixture_requires_explicit_initialization() {
    let temporary = tempfile::tempdir().unwrap();
    assert!(seed_files(&temporary).is_err());
    assert!(!temporary.path().join("pool/config.json").exists());
}
