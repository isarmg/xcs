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
    for (extra, expected) in [
        (vec![], 1),
        (vec!["--copies", "1"], 1),
        (vec!["--copies", "4"], 4),
        (vec!["--copies", "4294967295"], u32::MAX),
    ] {
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

#[test]
fn package_restore_rejects_zero_copies_as_an_argument_error() {
    let error = Args::try_parse_from([
        "zxs-rescue",
        "package",
        "restore",
        "/package",
        "--primary-slot",
        "P01",
        "--copies",
        "0",
    ])
    .err()
    .expect("zero copies cannot restore even the primary copy");
    assert_eq!(
        error.kind(),
        command_line::error::ErrorKind::ValueValidation
    );
    assert!(error.to_string().contains("--copies"));
}
