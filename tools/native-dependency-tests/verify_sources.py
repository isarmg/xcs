#!/usr/bin/env python3
"""Fail closed if an audited native fork or its documented patch drifts."""
import hashlib
import json
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

def verify(root=ROOT):
    base = root / "vendor/dependencies"
    records = json.loads((base / "native-provenance.json").read_text())
    for record in records:
        directory = base / record["directory"]
        if directory.is_symlink() or any(p.is_symlink() for p in directory.rglob("*")):
            raise ValueError(f"native source symlink not allowed: {record['directory']}")
        actual = {str(p.relative_to(directory)): hashlib.sha256(p.read_bytes()).hexdigest()
                  for p in directory.rglob("*") if p.is_file()}
        if actual != record["files_sha256"]:
            raise ValueError(f"native source integrity mismatch: {record['directory']}")
        patch = base / record["patch"]
        if patch.is_symlink():
            raise ValueError(f"native patch symlink not allowed: {record['directory']}")
        if hashlib.sha256(patch.read_bytes()).hexdigest() != record["patch_sha256"]:
            raise ValueError(f"native patch integrity mismatch: {record['directory']}")
    source = (base / "glib-0.18.5/src/variant_iter.rs").read_text()
    body = source.split("fn impl_get(&self, i: usize)", 1)[1].split("impl<'a> Iterator", 1)[0]
    if "let mut p: *mut libc::c_char" not in body or "&mut p," not in body or "&p," in body:
        raise ValueError("RUSTSEC-2024-0429 mutable out-argument backport missing")
    host = (root / "apps/host/Cargo.toml").read_text()
    for record in records:
        if f'path = "../../vendor/dependencies/{record["directory"]}"' not in host:
            raise ValueError("host no longer consumes verified native fork")
    # Verify both consumer manifests and lockfiles, even when the scanner omits
    # local path packages. The harness includes opener source directly.
    for consumer in ("apps/host", "tools/native-dependency-tests"):
        manifest = tomllib.loads((root / consumer / "Cargo.toml").read_text())
        lock = tomllib.loads((root / consumer / "Cargo.lock").read_text())
        overrides = dict(manifest.get("dependencies", {}))
        overrides.update(manifest.get("patch", {}).get("crates-io", {}))
        for record in records:
            name, version = record["directory"].rsplit("-", 1)
            if consumer == "tools/native-dependency-tests" and name == "tauri-plugin-opener":
                continue
            override = overrides.get(name, {})
            if not isinstance(override, dict) or override.get("path") != "../../vendor/dependencies/" + record["directory"]:
                raise ValueError(f"unverified consumer path: {consumer} {name}")
            entries = [p for p in lock["package"] if p["name"] == name]
            if len(entries) != 1 or entries[0]["version"] != version or "source" in entries[0] or "checksum" in entries[0]:
                raise ValueError(f"unverified consumer lock: {consumer} {name}")
    inventory = json.loads((base / "native-advisories.json").read_text())
    assert inventory[0]["id"] == "RUSTSEC-2024-0429"
    assert inventory[0]["status"] == "locally-backported-not-upstream-version-fixed"
    return inventory

if __name__ == "__main__":
    for item in verify():
        print(f"Native advisory inventory: {item['id']} {item['package']} {item['status']}")
    print("Verified exact native source trees and reviewed patch hashes; this is not a zero-advisory claim.")
