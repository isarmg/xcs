#!/usr/bin/env bash
# Pin the scanner, but fetch current RustSec advisories on every run.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
audit_root="$PWD/target/security-tools"
audit="$audit_root/bin/cargo-audit"
if [[ ! -x "$audit" ]] || [[ "$("$audit" --version)" != 'cargo-audit 0.22.2' ]]; then
  cargo install cargo-audit --version '=0.22.2' --locked --root "$audit_root" --force
fi
python3 .github/scripts/tests/test_security_gate.py
python3 tools/native-dependency-tests/verify_sources.py
python3 tools/native-dependency-tests/test_classify_audit.py >/dev/null
python3 tools/native-dependency-tests/test_upstream_audit.py >/dev/null
reports=$(mktemp -d)
trap 'rm -rf "$reports"' EXIT
# Keep stderr error detection shared by consumer and synthetic-upstream scans.
scan_lock() {
  local lockfile="$1"
  report="$reports/$(echo "$lockfile" | tr / _).json"
  diagnostics="$report.stderr"
  # cargo-audit can return zero after a failed yank lookup. Never interpret
  # its otherwise-clean JSON as a complete scan when diagnostics report errors.
  for attempt in 1 2; do
    scanner_status=0
    "$audit" audit --file "$lockfile" --deny warnings --json \
      > "$report" 2> "$diagnostics" || scanner_status=$?
    cat "$diagnostics" >&2
    if grep -qi '^error:' "$diagnostics"; then
      scanner_status=2
    fi
    if [[ "$scanner_status" -ne 2 || "$attempt" -eq 2 ]]; then
      break
    fi
    echo "Incomplete audit for $lockfile; retrying once against the official registry/database." >&2
  done
}
status=0
# Audit consumer lockfiles, not unused lockfiles shipped in upstream snapshots.
while IFS= read -r -d '' lockfile; do
  echo "Auditing $lockfile"
  scan_lock "$lockfile"
  if ! python3 tools/native-dependency-tests/classify_audit.py \
    "$report" "$lockfile" "$scanner_status"; then
    cat "$report"
    status=1
  fi
done < <(find Cargo.lock apps demos integrations tools web -name Cargo.lock \
  -not -path '*/target/*' -print0 | sort -z)
# RustSec skips path packages, so separately scan their original registry
# identities. Exact source/provenance checks and byte-for-byte shadow-lock
# validation permit only documented maintained forks, never future advisories.
upstream_lock="$reports/upstream.Cargo.lock"
if python3 tools/native-dependency-tests/upstream_audit.py build "$upstream_lock"; then
  echo 'Auditing vendored upstream identities'
  scan_lock "$upstream_lock"
  if ! python3 tools/native-dependency-tests/upstream_audit.py classify \
    "$report" "$upstream_lock" "$scanner_status"; then
    cat "$report"
    status=1
  fi
else
  status=1
fi
exit "$status"
