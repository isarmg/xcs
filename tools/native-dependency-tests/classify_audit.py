#!/usr/bin/env python3
"""Fail-closed audit adapter: no ignored IDs, exact local source classification only."""
import argparse
import json
import subprocess
import sys
import tomllib
from pathlib import Path
from verify_sources import ROOT, verify

ALLOWED = {
    ("RUSTSEC-2024-0429", "glib", "0.18.5"): "glib-0.18.5",
    ("RUSTSEC-2026-0173", "proc-macro-error2", "2.0.1"): "proc-macro-error2-2.0.1",
}

def classify(report, lockfile, scanner_status):
    if scanner_status not in (0, 1):
        raise ValueError(f"scanner failed with status {scanner_status}")
    # Require complete real scanner output, including warnings and enabled notices.
    if not report.get("database", {}).get("advisory-count"):
        raise ValueError("missing advisory database evidence")
    settings = report["settings"]
    if settings["ignore"] or set(settings["informational_warnings"]) != {"unmaintained", "unsound", "notice"}:
        raise ValueError("advisory warnings disabled or ignored")
    findings = list(report["vulnerabilities"]["list"])
    for entries in report["warnings"].values():
        findings.extend(entries)
    if scanner_status and not findings:
        raise ValueError("scanner failed without classifiable findings")
    metadata = None
    for finding in findings:
        advisory = finding.get("advisory") or {}
        package = finding.get("package") or {}
        key = (advisory.get("id"), package.get("name"), package.get("version"))
        if key not in ALLOWED or package.get("source") is not None:
            raise ValueError(f"unresolved audit finding: {key}")
        verify()
        packages = tomllib.loads(lockfile.read_text())["package"]
        matching = [p for p in packages if p["name"] == key[1] and p["version"] == key[2]]
        if len(matching) != 1 or "source" in matching[0] or "checksum" in matching[0]:
            raise ValueError("not the verified local package in lockfile")
        if metadata is None:
            metadata = json.loads(subprocess.check_output([
                "cargo", "metadata", "--manifest-path", str(lockfile.with_name("Cargo.toml")),
                "--format-version", "1", "--locked", "--offline"
            ]))
        resolved = [p for p in metadata["packages"] if p["name"] == key[1] and p["version"] == key[2]]
        expected = ROOT / "vendor/dependencies" / ALLOWED[key] / "Cargo.toml"
        if len(resolved) != 1 or Path(resolved[0]["manifest_path"]).resolve() != expected.resolve():
            raise ValueError("resolved package is not verified maintained fork")
        print(f"CLASSIFIED, NOT IGNORED: {key[0]} {key[1]} {key[2]}; exact local source verified; upstream advisory remains in inventory")
    if not findings:
        print(f"Scanner found no matching registry findings for {lockfile}; local maintenance inventory is checked separately.")

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("report", type=Path)
    parser.add_argument("lockfile", type=Path)
    parser.add_argument("scanner_status", type=int)
    args = parser.parse_args()
    try:
        classify(json.loads(args.report.read_text()), args.lockfile.resolve(), args.scanner_status)
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError) as error:
        print(f"Audit gate failed: {error}", file=sys.stderr)
        sys.exit(1)
