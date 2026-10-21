#!/usr/bin/env python3
"""Verify the single HTTP body package, feature isolation and retained behavior."""
import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
TARGETS = ("x86_64-unknown-linux-gnu", "wasm32-unknown-unknown")
FEATURE_SETS = ((), ("utilities",), ("channel",), ("full",))


def run(root, command, capture=False, succeeds=True):
    result = subprocess.run(
        command, cwd=root, text=True, capture_output=capture,
        check=succeeds, timeout=300,
    )
    if not succeeds and result.returncode == 0:
        raise AssertionError("HTTP body negative compilation unexpectedly succeeded")
    return result


def active_packages(metadata):
    active = {node["id"] for node in metadata["resolve"]["nodes"]}
    return {package["name"] for package in metadata["packages"] if package["id"] in active}


def validate_dependencies(metadata, features):
    names = active_packages(metadata)
    if "http-body-util" in names:
        raise AssertionError("absorbed utility package remains in the dependency graph")
    has_utilities = bool(features)
    for dependency in ("futures-core", "pin-project-lite"):
        if (dependency in names) != has_utilities:
            raise AssertionError(f"HTTP body utility dependency isolation failed: {dependency}")
    if ("tokio" in names) != bool(set(features) & {"channel", "full"}):
        raise AssertionError("HTTP body channel unexpectedly changed Tokio activation")


def check_minimal(root, temporary, target_dir):
    for index, features in enumerate(FEATURE_SETS):
        work = temporary / f"minimal-{index}"
        (work / "src").mkdir(parents=True)
        source = work / "src/lib.rs"
        source.write_text(
            "pub fn hint() -> http_body::SizeHint { http_body::SizeHint::with_exact(4) }\n",
            encoding="utf-8",
        )
        dependency = json.dumps(str(root / "src/foundation/http-body"))
        (work / "Cargo.toml").write_text(
            '[package]\nname = "http-body-feature-probe"\nversion = "0.0.0"\n'
            'edition = "2021"\n[workspace]\n[dependencies.http-body]\n'
            f'path = {dependency}\ndefault-features = false\nfeatures = {json.dumps(features)}\n'
            '[profile.dev]\ndebug = 0\nincremental = false\n',
            encoding="utf-8",
        )
        common = ["--offline", "--manifest-path", str(work / "Cargo.toml")]
        run(root, ["cargo", "generate-lockfile", *common])
        metadata = run(root, ["cargo", "metadata", "--format-version", "1", "--locked", *common], capture=True)
        validate_dependencies(json.loads(metadata.stdout), features)
        for target in TARGETS:
            command = [
                "cargo", "check", "--locked", *common, "--target", target,
                "--target-dir", str(target_dir), "--jobs", "2",
            ]
            run(root, command)
        if not features:
            source.write_text("use http_body::BodyExt;\n", encoding="utf-8")
            result = run(root, command, capture=True, succeeds=False)
            if "E0432" not in result.stderr or "BodyExt" not in result.stderr:
                raise AssertionError("minimal feature probe failed for an unrelated reason")


def check_retained_tests(root, temporary, target_dir):
    # A disposable manifest adds only already-maintained local test providers.
    # Product manifests and the root lock file are not modified.
    mirror = temporary / "retained"
    package = mirror / "src/foundation/http-body"
    shutil.copytree(root / "src/foundation/http-body", package)
    shutil.copytree(root / "test/foundation/http-body", mirror / "test/foundation/http-body")
    manifest = (package / "Cargo.toml").read_text(encoding="utf-8")
    manifest = re.sub(
        r'path = "\.\./([^"\n]+)"',
        lambda match: "path = " + json.dumps(str(root / "src/foundation" / match[1])),
        manifest,
    )
    manifest += (
        '\n[workspace]\n[dev-dependencies.futures-util]\n'
        f'path = {json.dumps(str(root / "src/foundation/futures-util"))}\nfeatures = ["std"]\n'
        '[dev-dependencies.tokio]\n'
        f'path = {json.dumps(str(root / "src/foundation/tokio"))}\nfeatures = ["full"]\n'
        '[[test]]\nname = "is_end_stream"\n'
        'path = "../../../test/foundation/http-body/tests/is_end_stream.rs"\n'
        '[profile.dev]\ndebug = 0\nincremental = false\n'
        '[profile.test]\ndebug = 0\nincremental = false\n'
    )
    (package / "Cargo.toml").write_text(manifest, encoding="utf-8")
    common = ["--offline", "--manifest-path", str(package / "Cargo.toml")]
    run(root, ["cargo", "generate-lockfile", *common])
    run(root, [
        "cargo", "test", "--locked", *common, "--all-features",
        "--target-dir", str(target_dir), "--jobs", "2",
    ])


def check(root):
    root = Path(root).resolve()
    for retired in ("src/foundation/http-body-util", "test/foundation/http-body-util"):
        if (root / retired).exists():
            raise AssertionError(f"absorbed HTTP body utility root remains: {retired}")
    temporary_root = root / "target/scope-consumers"
    temporary_root.mkdir(parents=True, exist_ok=True)
    target_dir = root / "target/http-body-consumers"
    with tempfile.TemporaryDirectory(prefix="http-body-", dir=temporary_root) as directory:
        temporary = Path(directory)
        check_minimal(root, temporary, target_dir)
        check_retained_tests(root, temporary, target_dir)
    run(root, [
        "cargo", "check", "--locked", "--offline", "--jobs", "2",
        "-p", "axum-core", "-p", "axum", "-p", "hyper", "-p", "tower-http", "-p", "reqwest",
    ])


if __name__ == "__main__":
    check(ROOT)
