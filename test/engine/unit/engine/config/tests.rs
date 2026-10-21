use super::*;

#[test]
fn public_validation_checks_paths_without_creating_them() {
    let root = tempfile::tempdir().unwrap();
    let mut config = Config::development([
        root.path().join("p"),
        root.path().join("b"),
        root.path().join("c"),
    ]);
    Engine::validate_configuration(&config).unwrap();
    for path in ["relative", "/draft/../p"] {
        config.disks[0].root = path.into();
        assert!(Engine::validate_configuration(&config).is_err());
    }
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn impossible_upstream_level_is_rejected_without_overflow() {
    let mut config = Config::development(["/draft/p".into(), "/draft/b".into(), "/draft/c".into()]);
    config.disks[0].level = 2;
    config.disks[0].upstream_slot_id = Some("B01".into());
    config.disks[1].level = u32::MAX;
    config.disks[1].upstream_slot_id = Some("C01".into());
    assert!(validate_config(&config).is_err());
}
