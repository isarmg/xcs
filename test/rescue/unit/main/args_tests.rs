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

const WORKSPACE_LIMIT_FLAGS: [&str; 4] = [
    "--max-retained-workspaces",
    "--max-retained-workspaces-per-principal",
    "--max-retained-workspace-metadata-bytes",
    "--max-retained-workspace-metadata-bytes-per-principal",
];

#[test]
fn retained_workspace_flags_require_all_positive_values_or_none() {
    let base = vec!["zxs-rescue", "package", "check", "/missing-package"];
    assert_eq!(
        Args::try_parse_from(base.clone())
            .unwrap()
            .workspace_limits()
            .unwrap(),
        None
    );
    for flag in WORKSPACE_LIMIT_FLAGS {
        let mut argv = base.clone();
        argv.extend([flag, "1"]);
        assert!(Args::try_parse_from(argv).is_err(), "{flag}");
    }
    for zero in 0..4 {
        let mut argv = base.clone();
        for (index, flag) in WORKSPACE_LIMIT_FLAGS.iter().enumerate() {
            argv.extend([*flag, if index == zero { "0" } else { "1" }]);
        }
        assert!(Args::try_parse_from(argv).is_err(), "zero dimension {zero}");
    }
    let mut argv = base;
    for (flag, value) in WORKSPACE_LIMIT_FLAGS.iter().zip(["7", "3", "11", "5"]) {
        argv.extend([*flag, value]);
    }
    let limits = Args::try_parse_from(argv)
        .unwrap()
        .workspace_limits()
        .unwrap()
        .unwrap();
    assert_eq!(
        limits,
        storage_engine::RetainedWorkspaceLimits {
            max_retained_workspaces: 7,
            max_retained_workspaces_per_principal: 3,
            max_retained_workspace_metadata_bytes: 11,
            max_retained_workspace_metadata_bytes_per_principal: 5,
        }
    );
}

#[test]
fn retained_workspace_flags_are_global_for_every_engine_command() {
    for command in [
        vec!["package", "export", "history", "/package"],
        vec!["package", "restore", "/package", "--primary-slot", "P01"],
        vec!["restore-system", "--disks", "/disk"],
        vec!["recovery-preview", "operation", "--reason", "review"],
        vec![
            "recovery-commit",
            "recovery",
            "--digest",
            "digest",
            "--accept-permanent-loss",
        ],
    ] {
        for before_command in [true, false] {
            let mut argv = vec!["zxs-rescue"];
            let limits = WORKSPACE_LIMIT_FLAGS
                .into_iter()
                .flat_map(|flag| [flag, "1"]);
            if before_command {
                argv.extend(limits);
                argv.extend(command.clone());
            } else {
                argv.extend(command.clone());
                argv.extend(limits);
            }
            assert!(
                Args::try_parse_from(argv)
                    .unwrap()
                    .workspace_limits()
                    .unwrap()
                    .is_some()
            );
        }
    }
}
