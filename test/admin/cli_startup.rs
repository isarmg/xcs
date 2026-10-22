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
        (vec!["--help"], "Full-file versioned storage and recovery"),
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
fn server_argument_still_selects_remote_dispatch() {
    // An invalid remote origin is rejected before any network I/O. Local
    // dispatch would instead fail trying to open a pool in the empty directory.
    let output = cli(&["--server", "file:///not-a-server", "whoami"], "1");
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.starts_with("zxs: "), "{output:?}");
    assert!(stderr.contains("use an HTTPS origin"), "{output:?}");
}
