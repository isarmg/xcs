use std::process::{Command, Output};

fn cli(args: &[&str], worker_threads: &str) -> Output {
    let directory = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_zxs"))
        .args(args)
        .current_dir(directory.path())
        .env("TOKIO_WORKER_THREADS", worker_threads)
        .env_remove("RUST_LOG")
        .env_remove("RUST_BACKTRACE")
        .output()
        .unwrap()
}

#[test]
fn informational_exits_do_not_construct_a_runtime() {
    // Zero is rejected when Tokio builds its multi-thread runtime. These are
    // real binary executions, not parser-only tests or a replacement runtime.
    for (args, expected) in [
        (
            vec!["--help"],
            "Local storage service, setup, administration and offline rescue",
        ),
        (
            vec!["--version"],
            concat!("zxs ", env!("CARGO_PKG_VERSION")),
        ),
        (vec!["config-template", "--help"], "--disks"),
    ] {
        let output = cli(&args, "0");
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        assert!(String::from_utf8_lossy(&output.stdout).contains(expected));
        assert!(output.stderr.is_empty(), "{output:?}");
    }
}

#[test]
fn argument_errors_do_not_construct_a_runtime() {
    for args in [vec!["--unknown-option"], vec![], vec!["config-template"]] {
        let output = cli(&args, "0");
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert!(output.stdout.is_empty(), "{output:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Usage:"), "{output:?}");
        assert!(!stderr.contains("TOKIO_WORKER_THREADS"), "{output:?}");
    }
}

#[test]
fn valid_arguments_still_construct_the_multi_thread_runtime() {
    let output = cli(
        &[
            "config-template",
            "--disks",
            "/disk-a",
            "/disk-b",
            "/disk-c",
        ],
        "0",
    );
    assert!(!output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("\"TOKIO_WORKER_THREADS\" cannot be set to 0"),
        "{output:?}"
    );
}

#[test]
fn valid_arguments_dispatch_and_preserve_configuration_output() {
    let output = cli(
        &[
            "config-template",
            "--production",
            "--disks",
            "/disk-a",
            "/disk-b",
            "/disk-c",
        ],
        "1",
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let config: storage_engine::Config = json_codec::from_slice(&output.stdout).unwrap();
    assert_eq!(config.mode, storage_engine::Mode::Production);
    assert_eq!(config.disks.len(), 3);
    for (disk, path) in config.disks.iter().zip(["/disk-a", "/disk-b", "/disk-c"]) {
        assert_eq!(disk.root, std::path::Path::new(path));
    }
}

#[test]
fn command_errors_keep_the_cli_error_prefix_and_exit_code() {
    let output = cli(
        &[
            "config-template",
            "--disks",
            "relative",
            "/disk-b",
            "/disk-c",
        ],
        "1",
    );
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "zxs: provide three distinct absolute disk roots\n"
    );
}

#[test]
fn removed_client_and_transfer_interfaces_fail_before_runtime_startup() {
    for args in [
        vec!["--server", "https://example.invalid", "repositories"],
        vec!["--token-file", "token", "repositories"],
        vec!["login"],
        vec!["logout"],
        vec!["whoami"],
        vec!["retry-commit", "request.json"],
        vec!["put", "branch", "path", "source"],
        vec!["get", "version", "output"],
        vec!["export", "version", "output"],
        vec![
            "historical-save",
            "branch",
            "commit",
            "name",
            "--source",
            "source",
            "--path",
            "path",
        ],
    ] {
        let output = cli(&args, "0");
        assert_eq!(output.status.code(), Some(2), "{args:?}: {output:?}");
        assert!(output.stdout.is_empty(), "{output:?}");
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("unrecognized subcommand") || error.contains("unexpected argument"),
            "{output:?}"
        );
        assert!(!error.contains("TOKIO_WORKER_THREADS"), "{output:?}");
    }
}

#[test]
fn service_and_rescue_commands_remain_available_without_opening_a_pool() {
    for command in [
        "serve",
        "init",
        "init-auth",
        "restore-system",
        "recovery-preview",
        "recovery-commit",
        "offline-check",
        "offline-export",
        "export-history",
        "check-history-package",
        "restore-history-package",
        "account",
        "smb-binding",
    ] {
        let output = cli(&[command, "--help"], "0");
        assert_eq!(output.status.code(), Some(0), "{command}: {output:?}");
        assert!(output.stderr.is_empty(), "{output:?}");
    }
}

#[test]
fn initialization_and_system_rescue_preserve_repository_and_credentials() {
    use std::{fs, os::unix::fs::PermissionsExt};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("system");
    let replacement = temp.path().join("replacement");
    let disks = [
        temp.path().join("a"),
        temp.path().join("b"),
        temp.path().join("c"),
    ];
    let password = temp.path().join("password");
    fs::write(&password, b"test-only-password").unwrap();
    fs::set_permissions(&password, fs::Permissions::from_mode(0o600)).unwrap();
    let run = |root: &std::path::Path, args: &[&std::ffi::OsStr]| {
        let output = Command::new(env!("CARGO_BIN_EXE_zxs"))
            .arg("--root")
            .arg(root)
            .args(args)
            .env("TOKIO_WORKER_THREADS", "1")
            .env_remove("RUST_LOG")
            .env_remove("RUST_BACKTRACE")
            .output()
            .unwrap();
        assert!(output.status.success(), "{args:?}: {output:?}");
        json_codec::from_slice::<json_codec::Value>(&output.stdout).unwrap()
    };
    run(
        &root,
        &[
            "init".as_ref(),
            "--username".as_ref(),
            "admin".as_ref(),
            "--password-file".as_ref(),
            password.as_os_str(),
            "--development-disks".as_ref(),
            disks[0].as_os_str(),
            disks[1].as_os_str(),
            disks[2].as_os_str(),
        ],
    );
    run(
        &root,
        &[
            "create-repo".as_ref(),
            "rescue-fixture".as_ref(),
            "--primary-slot".as_ref(),
            "P01".as_ref(),
        ],
    );
    let before = run(&root, &["repositories".as_ref()]);
    run(&root, &["check".as_ref()]);
    fs::remove_dir_all(&root).unwrap();
    let restored = run(
        &replacement,
        &[
            "restore-system".as_ref(),
            "--disks".as_ref(),
            disks[0].as_os_str(),
            disks[1].as_os_str(),
            disks[2].as_os_str(),
        ],
    );
    assert_eq!(restored["credentials_restored"], true);
    assert_eq!(run(&replacement, &["repositories".as_ref()]), before);
    run(&replacement, &["check".as_ref()]);
    run(
        &replacement,
        &["offline-check".as_ref(), disks[0].as_os_str()],
    );
}
