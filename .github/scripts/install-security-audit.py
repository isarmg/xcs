#!/usr/bin/env python3
"""Build the pinned scanner with reviewed, independently locked dependency fixes."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import tomllib
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
CONFIG = ROOT / ".github/security-tools"
VERSION = "cargo-audit 0.22.2"
REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"


def digest(data):
    return hashlib.sha256(data).hexdigest()


def verify(data, expected, label):
    if digest(data) != expected:
        raise ValueError(f"{label}: SHA-256 mismatch")


def validate_lock(original, patched, recipe):
    verify(original, recipe["original_lock_sha256"], "original tool lock")
    verify(patched, recipe["patched_lock_sha256"], "patched tool lock")
    before = tomllib.loads(original.decode())
    after = tomllib.loads(patched.decode())
    def trusted_source(package):
        return (package is None or package.get("source") == REGISTRY
                or (package.get("name") == "cargo-audit"
                    and package.get("version") == "0.22.2" and "source" not in package))

    for change in recipe["lock_changes"]:
        old, new = change["before"], change["after"]
        if not trusted_source(new) or not trusted_source(old):
            raise ValueError("unexpected tool dependency source")
        if old is not None:
            if before["package"].count(old) != 1:
                raise ValueError("original lock does not match approved dependency change")
            before["package"].remove(old)
        if new is not None:
            before["package"].append(new)
    package_key = lambda package: (package["name"], package["version"], package.get("source", ""))
    before["package"].sort(key=package_key)
    after["package"].sort(key=package_key)
    if before != after:
        raise ValueError("tool lock changes more than the approved dependency closure")


def cached(binary, stamp, identity):
    try:
        recorded = json.loads(stamp.read_text())
        return (recorded["recipe_sha256"] == identity
                and recorded["binary_sha256"] == digest(binary.read_bytes())
                and subprocess.check_output([str(binary), "--version"], text=True).strip() == VERSION)
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError):
        return False


def download(url, destination):
    with urllib.request.urlopen(url, timeout=120) as response:
        destination.write_bytes(response.read())


def install(config=CONFIG, root=ROOT / "target/security-tools"):
    recipe_bytes = (config / "cargo-audit.json").read_bytes()
    recipe = json.loads(recipe_bytes)
    patched = (config / "cargo-audit-0.22.2.lock").read_bytes()
    verify(patched, recipe["patched_lock_sha256"], "patched tool lock")
    toolchain_bytes = (ROOT / "rust-toolchain.toml").read_bytes()
    toolchain = tomllib.loads(toolchain_bytes.decode())["toolchain"]["channel"]
    identity = digest(Path(__file__).read_bytes() + recipe_bytes + patched + toolchain_bytes)
    binary = root / "bin/cargo-audit"
    stamp = root / "verified-build.json"
    if cached(binary, stamp, identity):
        return binary
    root.mkdir(parents=True, exist_ok=True)
    # Never build in the product tree: its offline source replacement is not
    # the online scanner's registry. Only build outputs live in ignored target/.
    with tempfile.TemporaryDirectory(prefix="zxs-security-tool-") as directory:
        context = Path(directory)
        archive = context / "cargo-audit.crate"
        download(recipe["archive_url"], archive)
        verify(archive.read_bytes(), recipe["archive_sha256"], "official cargo-audit archive")
        with tarfile.open(archive, "r:gz") as source:
            source.extractall(context, filter="data")
        source = context / "cargo-audit-0.22.2"
        lock = source / "Cargo.lock"
        validate_lock(lock.read_bytes(), patched, recipe)
        lock.write_bytes(patched)
        verify(lock.read_bytes(), recipe["patched_lock_sha256"], "installed tool lock")
        environment = os.environ.copy()
        environment["CARGO_TARGET_DIR"] = str(root / "build")
        environment["RUSTUP_TOOLCHAIN"] = toolchain
        subprocess.run(["cargo", "install", "--path", str(source), "--locked",
                        "--root", str(root), "--force", "--jobs", "2"],
                       cwd=context, env=environment, check=True)
        # --locked must not rewrite the reviewed resolution.
        verify(lock.read_bytes(), recipe["patched_lock_sha256"], "built tool lock")
    if subprocess.check_output([str(binary), "--version"], text=True).strip() != VERSION:
        raise ValueError("installed scanner reports an unexpected version")
    state = {"recipe_sha256": identity, "binary_sha256": digest(binary.read_bytes())}
    temporary = stamp.with_suffix(".tmp")
    temporary.write_text(json.dumps(state, indent=2) + "\n")
    temporary.replace(stamp)
    return binary


if __name__ == "__main__":
    try:
        install()
    except (OSError, ValueError, KeyError, subprocess.SubprocessError, tarfile.TarError) as error:
        print(f"Security scanner installation failed: {error}", file=sys.stderr)
        sys.exit(1)
