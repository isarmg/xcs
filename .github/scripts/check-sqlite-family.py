#!/usr/bin/env python3
"""Run preserved SQLite tests and compiler boundaries without changing root inputs."""
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[2]
INTEGRATION_TESTS = {
    "hash_table_compatibility": "test/engine/integration/hash_table_compatibility.rs",
    "sqlite_foundation": "test/platform/integration/sqlite_foundation.rs",
}


def cargo_command(operation, manifest, target, *arguments):
    return ["cargo", operation, "--locked", "--offline", "--color", "never", "--manifest-path", str(manifest),
            "--target-dir", str(target), *arguments]


def execute(command, root, expected=None, minimum_tests=None):
    result = subprocess.run(command, cwd=root, text=True, capture_output=True)
    if expected is not None:
        diagnostic = re.sub(r"\x1b\[[0-?]*[ -/]*[@-~]", "", result.stderr)
        assert result.returncode and expected in diagnostic, (command, result.stdout, result.stderr)
        return
    assert result.returncode == 0, (command, result.stdout, result.stderr)
    print(result.stdout, end="", flush=True)
    print(result.stderr, end="", flush=True)
    if minimum_tests is not None:
        passed = sum(map(int, re.findall(r"test result: ok\. (\d+) passed;", result.stdout)))
        assert passed >= minimum_tests, f"expected at least {minimum_tests} real tests, got {passed}"


def lock(manifest, root):
    execute(["cargo", "generate-lockfile", "--offline", "--manifest-path", str(manifest)], root)


def prepare_package(root, work):
    """Only the maintained package/tests are copied; dependencies remain real providers."""
    original = root / "src/foundation/sqlite-access"
    package = work / "src/foundation/sqlite-access"
    shutil.copytree(original, package, ignore=shutil.ignore_patterns("Cargo.lock", "target", "__pycache__"))
    tests = "test/foundation/sqlite-access"
    shutil.copytree(root / tests, work / tests)
    manifest = package / "Cargo.toml"
    text = manifest.read_text()
    parsed = tomllib.loads(text)
    for section in ("dependencies", "dev-dependencies", "build-dependencies"):
        for entry in parsed.get(section, {}).values():
            if isinstance(entry, dict) and "path" in entry:
                old = entry["path"]
                text = text.replace(f'path = "{old}"', f'path = {json.dumps(str((original / old).resolve()))}')
    if "readme" in parsed["package"]:
        old = parsed["package"]["readme"]
        text = text.replace(f'readme = "{old}"', f'readme = {json.dumps(str((original / old).resolve()))}')
    assert "[workspace]" not in text
    manifest.write_text(text + "\n[workspace]\n")
    return manifest


def consumer_manifest(root, work, features):
    manifest = work / "Cargo.toml"
    (work / "src").mkdir(parents=True, exist_ok=True)
    text = ('[package]\nname="sqlite-family-consumer"\nversion="0.0.0"\nedition="2021"\n'
            '[workspace]\n[dependencies]\n'
            f'sqlite-access={{path={json.dumps(str(root / "src/foundation/sqlite-access"))},default-features=false,features={json.dumps(features)}}}\n'
            f'tokio={{path={json.dumps(str(root / "src/foundation/tokio"))},default-features=false,features=["macros","rt","time","sync"]}}\n'
            f'hash-table={{path={json.dumps(str(root / "src/foundation/hash-table"))}}}\n'
            f'hashlink={{path={json.dumps(str(root / "src/foundation/hashlink"))}}}\n')
    for name, path in INTEGRATION_TESTS.items():
        text += f'[[test]]\nname={json.dumps(name)}\npath={json.dumps(str(root / path))}\n'
    manifest.write_text(text)
    return manifest


def check(root):
    root = root.resolve()
    root_lock = root / "Cargo.lock"
    original_lock = root_lock.read_bytes()
    source = root / "src/foundation/sqlite-access"
    original_source = {p.relative_to(source).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
                       for p in source.rglob("*") if p.is_file()}
    (root / "target").mkdir(exist_ok=True)
    target = root / "target/sqlite-family-check"
    try:
        with tempfile.TemporaryDirectory(prefix="sqlite-family-", dir=root / "target") as temporary:
            work = Path(temporary)
            package_manifest = prepare_package(root, work)
            lock(package_manifest, root)
            execute(cargo_command("test", package_manifest, target, "--lib", "--all-features"), root, minimum_tests=12)
            consumer = work / "consumer"
            manifest = consumer_manifest(root, consumer, ["sqlite-bundled"])
            (consumer / "src/lib.rs").write_text("pub fn accepts(_: Option<sqlite_access::SqlitePool>) {}\n")
            lock(manifest, root)
            execute(cargo_command("test", manifest, target, "--tests"), root, minimum_tests=8)
            for features in ([], ["sqlite-bundled"]):
                manifest = consumer_manifest(root, consumer, features)
                lock(manifest, root)
                execute(cargo_command("check", manifest, target, "--lib"), root)
            (consumer / "src/lib.rs").write_text("use sqlite_access::query_engine;\n")
            execute(cargo_command("check", manifest, target, "--lib"), root, expected="error[E0603]")
            query = package_manifest.parent / "src/query_engine/mod.rs"
            before = query.read_bytes()
            try:
                query.write_bytes(before + b"\npub fn forbidden_probe() { unsafe {} }\n")
                execute(cargo_command("check", package_manifest, target, "--lib", "--all-features"), root,
                        expected="usage of an `unsafe` block")
            finally:
                query.write_bytes(before)
            execute(cargo_command("check", package_manifest, target, "--lib", "--all-features"), root)
    finally:
        assert root_lock.read_bytes() == original_lock, "SQLite gate changed the root lock"
        current = {p.relative_to(source).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
                   for p in source.rglob("*") if p.is_file()}
        assert current == original_source, "SQLite gate changed maintained source files"
    print("SQLite family: 12+ library tests, 8+ preserved integration tests, minimal/bundled consumers and private/unsafe compiler boundaries verified; root lock and source unchanged.")


if __name__ == "__main__":
    check(ROOT)
