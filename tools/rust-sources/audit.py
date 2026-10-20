#!/usr/bin/env python3
"""Audit consumer locks and original identities of local source forks without exemptions."""
import argparse
import json
from pathlib import Path
import re
import sys
import tomllib

import manage


def identities(catalog):
    packages = {}
    # All active Cargo packages are paths now, so consumer locks no longer
    # contain registry identities. Scan all retained implementations, including
    # merged and optional code. A verified retirement removes the source and its
    # dependency edges; its immutable origin stays in the historical inventory.
    originals = None if catalog.get("schema", 0) >= 3 else catalog.get("upstream_identities")
    if originals is None:
        originals = [package["upstream"] for package in catalog["packages"].values()]
        originals += [merge["upstream"] for merge in catalog.get("merges", [])]
    for upstream in originals:
        name, version, checksum = (upstream[key] for key in ("name", "version", "archive_sha256"))
        if not re.fullmatch(r"[a-f0-9]{64}", checksum):
            raise ValueError(f"invalid upstream checksum: {name}@{version}")
        identity = dict(name=name, version=version, source=manage.REGISTRY, checksum=checksum)
        key = name, version
        if key in packages and packages[key] != identity:
            raise ValueError(f"conflicting upstream identities: {key}")
        packages[key] = identity
    return [packages[key] for key in sorted(packages)]


def render(packages):
    return "# Original source identities; advisory matching only, NEVER a build lock.\nversion = 4\n" + "".join(
        "\n[[package]]\n" + "".join(f"{key} = {json.dumps(package[key])}\n"
                                    for key in ("name", "version", "source", "checksum"))
        for package in packages)


def classify(report, lockfile, scanner_status):
    if scanner_status not in (0, 1):
        raise ValueError(f"incomplete scanner execution: status {scanner_status}")
    count = len(tomllib.loads(lockfile.read_text(encoding="utf-8"))["package"])
    if not report.get("database", {}).get("advisory-count") or report["lockfile"]["dependency-count"] != count:
        raise ValueError("incomplete advisory database or package coverage")
    settings = report["settings"]
    if settings["ignore"] or settings.get("target_arch") or settings.get("target_os"):
        raise ValueError("advisory ignores or target exclusions are forbidden")
    if set(settings["informational_warnings"]) != {"unmaintained", "unsound", "notice"}:
        raise ValueError("informational advisory categories were disabled")
    findings = list(report["vulnerabilities"]["list"])
    for warnings in report["warnings"].values():
        findings.extend(warnings)
    if findings or report["vulnerabilities"]["found"] or report["vulnerabilities"]["count"] or scanner_status:
        labels = [f"{finding.get('advisory', {}).get('id', 'registry-warning') if finding.get('advisory') else 'registry-warning'}: "
                  f"{finding.get('package', {}).get('name', 'unknown')}" for finding in findings]
        raise ValueError("unresolved advisory findings: " + (", ".join(labels) or "inconsistent scanner result"))
    print(f"Complete advisory scan: {count} package identities, no findings.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    subcommands = parser.add_subparsers(dest="command", required=True)
    build = subcommands.add_parser("build")
    build.add_argument("lockfile", type=Path)
    check = subcommands.add_parser("check")
    check.add_argument("report", type=Path)
    check.add_argument("lockfile", type=Path)
    check.add_argument("scanner_status", type=int)
    arguments = parser.parse_args()
    try:
        if arguments.command == "build":
            manage.verify(manage.ROOT)
            catalog = manage.read_json(manage.ROOT / manage.CATALOG)
            arguments.lockfile.write_text(render(identities(catalog)), encoding="utf-8", newline="\n")
        else:
            classify(manage.read_json(arguments.report), arguments.lockfile, arguments.scanner_status)
    except (ValueError, OSError, KeyError, TypeError) as error:
        print(f"Rust advisory gate failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
