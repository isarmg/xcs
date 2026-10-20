#!/usr/bin/env python3
"""Audit original identities of local forks; never change a consumer lockfile."""
import argparse
import importlib.util
import json
import re
import sys
import tomllib
from pathlib import Path
from verify_sources import ROOT, verify

REGISTRY = "registry+https://github.com/rust-lang/crates.io-index"
NATIVE = {"glib", "glib-macros", "gtk3-macros", "tauri-plugin-opener", "proc-macro-error2", "proc-macro-error-attr2"}
LEPTOS = {"any_spawner", "const_str_slice_concat", "either_of", "leptos_dom", "leptos_hot_reload", "next_tuple", "oco_ref", "or_poisoned", "reactive_graph", "tachys", "throw_error"}
REVIEWED = {
    ("RUSTSEC-2024-0429", "glib", "0.18.5"): "locally-backported-not-upstream-version-fixed",
    ("RUSTSEC-2026-0173", "proc-macro-error2", "2.0.1"): "repository-maintained-upstream-unmaintained",
}

def identities():
    verify()
    spec = importlib.util.spec_from_file_location("leptos_vendor", ROOT / "tools/vendor-leptos/vendor.py")
    vendor = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(vendor)
    vendor.verify(False)
    native = json.loads((ROOT / "vendor/dependencies/native-provenance.json").read_text())
    web = json.loads((ROOT / "vendor/web/dependencies/provenance.json").read_text())["packages"]
    renamed = json.loads((ROOT / "vendor/web/upstream-audit-identities.json").read_text())
    extra = json.loads((ROOT / "vendor/dependencies/upstream-audit-identities.json").read_text())
    if {r["name"] for r in extra} != {"unicode-normalization"}:
        raise ValueError("existing Unicode fork identity missing")
    if {r["directory"].rsplit("-", 1)[0] for r in native} != NATIVE or set(web) != LEPTOS:
        raise ValueError("upstream inventory missing or changed native/Leptos family identity")
    if {r["name"] for r in renamed} != {"leptos", "leptos_macro"}:
        raise ValueError("renamed framework identity missing")
    result = []
    for r in native:
        name, version = r["directory"].rsplit("-", 1)
        result.append(dict(name=name, version=version, source=REGISTRY, checksum=r["archive_sha256"]))
    for name, r in web.items():
        result.append(dict(name=name, version=r["version"], source=r["registry_source"], checksum=r["archive_sha256"]))
    for r in renamed + extra:
        manifest = tomllib.loads((ROOT / r["maintained_directory"] / "Cargo.toml").read_text())["package"]
        if manifest["name"] != r["maintained_name"] or manifest["version"] != r["version"]:
            raise ValueError("renamed framework lineage mismatch; update audit mapping after review")
        result.append(dict(name=r["name"], version=r["version"], source=REGISTRY, checksum=r["archive_sha256"]))
    if len(result) != 20 or len({r["name"] for r in result}) != 20:
        raise ValueError("upstream inventory must contain all 20 distinct fork identities")
    for r in result:
        if r["source"] != REGISTRY or not re.fullmatch(r"[a-f0-9]{64}", r["checksum"]):
            raise ValueError("invalid upstream registry identity/checksum")
    # A future vendored dependency must not silently disappear from advisory
    # matching. All current first-party package names use the zxs prefix.
    covered = {(r["name"], r["version"]) for r in result}
    locks = [ROOT / "Cargo.lock"]
    for directory in ("apps", "demos", "integrations", "tools", "web"):
        locks.extend(p for p in (ROOT / directory).rglob("Cargo.lock") if "target" not in p.relative_to(ROOT).parts)
    validate_coverage(locks, covered)
    return sorted(result, key=lambda r: r["name"])

def validate_coverage(locks, covered):
    for lock in locks:
        for package in tomllib.loads(lock.read_text())["package"]:
            if "source" not in package and not package["name"].startswith("zxs"):
                if (package["name"], package["version"]) not in covered:
                    raise ValueError(f"uncovered local dependency in {lock}: {package['name']} {package['version']}")

def render(packages):
    return "# Synthetic upstream advisory inventory; NEVER used for a build.\nversion = 4\n" + "".join(
        "\n[[package]]\n" + "".join(f"{key} = {json.dumps(p[key])}\n" for key in ("name", "version", "source", "checksum"))
        for p in packages)

def validate_shadow(path, packages):
    if path.read_text() != render(packages):
        raise ValueError("shadow audit lock was omitted from, changed, or tampered with")

def classify(report, path, scanner_status):
    packages = identities()
    validate_shadow(path, packages)
    if scanner_status not in (0, 1):
        raise ValueError(f"incomplete upstream scan: status {scanner_status}")
    if not report.get("database", {}).get("advisory-count") or report["lockfile"]["dependency-count"] != len(packages):
        raise ValueError("scanner did not audit the complete upstream inventory")
    settings = report["settings"]
    if settings["ignore"] or set(settings["informational_warnings"]) != {"unmaintained", "unsound", "notice"}:
        raise ValueError("upstream advisories were ignored or informational warnings disabled")
    findings = list(report["vulnerabilities"]["list"])
    for entries in report["warnings"].values():
        findings.extend(entries)
    if scanner_status and not findings:
        raise ValueError("scanner failed without reported findings")
    inventory = json.loads((ROOT / "vendor/dependencies/native-advisories.json").read_text())
    approved_statuses = {(r["id"], r["status"]) for r in inventory}
    expected = {r["name"]: r for r in packages}
    for finding in findings:
        advisory, package = finding.get("advisory") or {}, finding.get("package") or {}
        key = advisory.get("id"), package.get("name"), package.get("version")
        status = REVIEWED.get(key)
        if status is None or (key[0], status) not in approved_statuses:
            raise ValueError(f"NEW OR UNREVIEWED upstream advisory on maintained source: {key}")
        wanted = expected.get(key[1], {})
        if any(package.get(field) != wanted.get(field) for field in ("name", "version", "source", "checksum")):
            raise ValueError("finding does not match exact verified upstream identity")
        print(f"UPSTREAM ADVISORY RETAINED: {key[0]} {key[1]} {key[2]}: {status}; source verified, not ignored")
    print(f"Audited original registry identities for all {len(packages)} local forks; {len(findings)} explicitly reviewed upstream findings retained.")

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    build = sub.add_parser("build")
    build.add_argument("lockfile", type=Path)
    check = sub.add_parser("classify")
    check.add_argument("report", type=Path)
    check.add_argument("lockfile", type=Path)
    check.add_argument("scanner_status", type=int)
    args = parser.parse_args()
    try:
        if args.command == "build":
            # Exclusive creation prevents accidental overwrite of real locks.
            with args.lockfile.open("x") as output:
                output.write(render(identities()))
        else:
            classify(json.loads(args.report.read_text()), args.lockfile, args.scanner_status)
    except (ValueError, KeyError, OSError) as error:
        print(f"Upstream fork audit failed: {error}", file=sys.stderr)
        sys.exit(1)
