//! Prove that ignored evidence is optional in an exported source tree, without
//! weakening mandatory source links or changing the real repository's index.
use std::{
    collections::BTreeSet,
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn git(root: &Path) -> Command {
    let mut command = Command::new("git");
    command.args(["-c", "core.excludesFile=", "-C"]).arg(root);
    command
}
#[test]
fn source_export_checks_without_generated_evidence_and_still_rejects_broken_source_links() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let destination =
        std::env::temp_dir().join(format!("xcs-source-export-{}-{nonce}", std::process::id()));
    fs::create_dir(&destination).unwrap();
    let fixture = Fixture(destination);
    let files = git(&root)
        .args([
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ])
        .output()
        .expect("Git is required for source export verification");
    assert!(
        files.status.success(),
        "{}",
        String::from_utf8_lossy(&files.stderr)
    );
    let mut ignored = git(&root)
        .args(["check-ignore", "--no-index", "-z", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    ignored
        .stdin
        .take()
        .unwrap()
        .write_all(&files.stdout)
        .unwrap();
    let ignored = ignored.wait_with_output().unwrap();
    assert!(
        matches!(ignored.status.code(), Some(0 | 1)),
        "{}",
        String::from_utf8_lossy(&ignored.stderr)
    );
    let excluded: BTreeSet<_> = ignored
        .stdout
        .split(|b| *b == 0)
        .filter(|p| !p.is_empty())
        .collect();
    for raw in files
        .stdout
        .split(|b| *b == 0)
        .filter(|p| !p.is_empty() && !excluded.contains(p))
    {
        let name = std::str::from_utf8(raw).unwrap();
        let relative = Path::new(name);
        assert!(
            relative
                .components()
                .all(|part| matches!(part, Component::Normal(_))),
            "{name}"
        );
        let source = root.join(relative);
        assert!(
            source.symlink_metadata().unwrap().file_type().is_file(),
            "export requires regular source files: {name}"
        );
        let output = fixture.0.join(relative);
        fs::create_dir_all(output.parent().unwrap()).unwrap();
        fs::copy(source, output).unwrap();
    }
    assert!(fixture.0.join("docs/contracts/vectors.json").is_file());
    assert!(
        fixture
            .0
            .join("tests/evidence/merge-tags-backup/README.md")
            .is_file()
    );
    assert!(
        fixture
            .0
            .join("demos/branch-system/tests/browser.mjs")
            .is_file()
    );
    assert!(
        !fixture
            .0
            .join("tests/evidence/merge-tags-backup/candidate-receipt.json")
            .exists()
    );
    assert!(
        !fixture
            .0
            .join("demos/branch-system/tests/evidence/browser-result.json")
            .exists()
    );
    let result = Command::new(env!("CARGO_BIN_EXE_xcs-contract-check"))
        .arg("--root")
        .arg(&fixture.0)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["result"], "PASS");
    assert!(
        report["missing_local_generated_artifacts"]
            .as_u64()
            .unwrap()
            > 0
    );
    fs::write(
        fixture.0.join("docs/source-link-regression.md"),
        "[Required source](missing-required-source.rs)\n",
    )
    .unwrap();
    let failure = Command::new(env!("CARGO_BIN_EXE_xcs-contract-check"))
        .arg("--root")
        .arg(&fixture.0)
        .output()
        .unwrap();
    assert!(!failure.status.success());
    assert!(String::from_utf8_lossy(&failure.stderr).contains("missing-required-source.rs"));
}
