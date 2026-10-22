use super::*;

#[test]
fn restore_system_accepts_one_original_member_but_requires_a_path() {
    let args =
        Args::try_parse_from(["zxs-rescue", "restore-system", "--disks", "/disk/only"]).unwrap();
    let Command::RestoreSystem { disks } = args.command else {
        panic!("wrong command")
    };
    assert_eq!(disks, [PathBuf::from("/disk/only")]);
    assert!(Args::try_parse_from(["zxs-rescue", "restore-system", "--disks"]).is_err());
}

#[test]
fn package_restore_defaults_to_one_copy_and_accepts_explicit_backup_policy() {
    for (extra, expected) in [(vec![], 1), (vec!["--copies", "4"], 4)] {
        let mut argv = vec![
            "zxs-rescue",
            "package",
            "restore",
            "/package",
            "--primary-slot",
            "P01",
        ];
        argv.extend(extra);
        let args = Args::try_parse_from(argv).unwrap();
        let Command::Package {
            command: PackageCommand::Restore { copies, .. },
        } = args.command
        else {
            panic!("wrong command")
        };
        assert_eq!(copies, expected);
    }
}
