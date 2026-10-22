#!/usr/bin/env python3
"""Build and execute real Linux C/C++/assembler consumers of the internal cc."""
from pathlib import Path
import base64
import gzip
import hashlib
import json
import tomllib
import runpy
import sys
import os
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]


def check_preservation():
    archive = ROOT / "compliance/cc-1.8.0-specialization.json.gz"
    content = archive.read_bytes()
    assert hashlib.sha256(content).hexdigest() == "859c778bea68dc448e1d44b56c477cc38b50e3b22d6c90d7b6ab773c3f1a8c8c", "C source preservation changed"
    record = json.loads(gzip.decompress(content))
    package = record["package"]
    expected = {package["directory"] + "/" + path: digest for path, digest in package["files_sha256"].items()}
    expected.update({item["path"]: item["sha256"] for item in record["external_tests"]["files"]})
    assert set(record["files"]) == set(expected)
    for path, digest in expected.items():
        assert hashlib.sha256(base64.b64decode(record["files"][path], validate=True)).hexdigest() == digest, path
    for path, digest in record["license_materials_sha256"].items():
        assert hashlib.sha256((ROOT / path).read_bytes()).hexdigest() == digest, path
    manifest = tomllib.loads((ROOT / "src/foundation/cc/Cargo.toml").read_text())
    assert not {"parallel", "jobserver"} & manifest.get("features", {}).keys()
    assert "find-msvc-tools" not in manifest.get("dependencies", {})
    # Existing guards do not permit partial source retirement. Keep these
    # original files byte-exact but unreachable, without advertising a feature.
    for path, digest in expected.items():
        if path.startswith("src/foundation/cc/src/parallel/"):
            assert hashlib.sha256((ROOT / path).read_bytes()).hexdigest() == digest, path
    active_source = (ROOT / "src/foundation/cc/src/lib.rs").read_text()
    assert "mod parallel" not in active_source and "parallel::" not in active_source
    assert not (ROOT / "src/foundation/find-msvc-tools").exists()


def check():
    check_preservation()
    with tempfile.TemporaryDirectory(prefix="zxs-native-c-") as temporary:
        work = Path(temporary)
        (work / "src").mkdir()
        (work / "Cargo.toml").write_text(f'''[package]
name = "native-c-toolchain-regression"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
cc = {{ path = "{ROOT / 'src/foundation/cc'}" }}
shell-tokenizer = {{ path = "{ROOT / 'src/foundation/shell-tokenizer'}" }}
blake3 = {{ path = "{ROOT / 'src/foundation/blake3'}" }}
''')
        (work / "src/main.rs").write_bytes((ROOT / "test/ci/fixtures/native-c-toolchain.rs").read_bytes())
        environment = os.environ | {"CARGO_BUILD_JOBS": "1", "CARGO_PROFILE_DEV_DEBUG": "0"}
        diagnostics_type = runpy.run_path(str(ROOT / ".github/scripts/check-rust-diagnostics.py"))["CargoDiagnostics"]
        diagnostics = diagnostics_type(sys.stderr)
        artifacts = {}
        command = ["cargo", "build", "--offline", "--message-format=json", "--manifest-path", str(work / "Cargo.toml")]
        with subprocess.Popen(command, cwd=work, env=environment, stdout=subprocess.PIPE, text=True) as process:
            for number, line in enumerate(process.stdout, 1):
                diagnostics.consume(line, number)
                try:
                    event = json.loads(line)
                except ValueError:
                    continue  # The shared strict checker rejects malformed events.
                if event.get("reason") == "compiler-artifact":
                    for filename in event.get("filenames", []):
                        if filename.endswith(".rlib"):
                            artifacts[event["target"]["name"]] = filename
                    if event.get("executable"):
                        artifacts[event["target"]["name"]] = event["executable"]
            if diagnostics.result(process.wait()):
                raise RuntimeError("native toolchain build diagnostics rejected")
        subprocess.run([artifacts["native-c-toolchain-regression"]], cwd=work, env=environment, check=True)
        # This standalone manifest tests the exact retained upstream unit tests
        # and doctests; -p cc at the root hits a Cargo build-dependency resolver bug.
        cc_manifest = ROOT / "src/foundation/cc/Cargo.toml"
        nested_lock = cc_manifest.with_name("Cargo.lock")
        had_lock = nested_lock.exists()
        try:
            subprocess.run(["cargo", "test", "--offline", "--manifest-path", str(cc_manifest)],
                           cwd=ROOT, env=environment, check=True)
        finally:
            if not had_lock:
                nested_lock.unlink(missing_ok=True)
        # Removed features must fail resolution rather than advertise unusable code.
        for feature in ("parallel", "jobserver"):
            result = subprocess.run(["cargo", "check", "--offline", "--features", f"cc/{feature}",
                                     "--manifest-path", str(work / "Cargo.toml")],
                                    cwd=work, env=environment, capture_output=True, text=True)
            assert result.returncode != 0 and "does not have" in result.stderr and feature in result.stderr, result.stderr


if __name__ == "__main__":
    check()
