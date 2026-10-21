#!/usr/bin/env bash
# Pin the scanner, but fetch current RustSec advisories on every run.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
audit_root="$PWD/target/security-tools"
audit="$audit_root/bin/cargo-audit"
python3 test/ci/test_security_audit_install.py
python3 .github/scripts/install-security-audit.py
python3 test/ci/test_security_gate.py
python3 -m unittest discover -s test/tools/rust-sources -p 'test_audit.py'
reports=$(mktemp -d)
trap 'rm -rf "$reports"' EXIT
# cargo-audit selects this cwd-local policy instead of any inherited audit.toml.
mkdir -p "$reports/.cargo"
cp .github/security-tools/audit.toml "$reports/.cargo/audit.toml"
# Keep stderr error detection shared by consumer and synthetic-upstream scans.
scan_lock() {
  local lockfile="$1"
  local scan_path="$lockfile"
  [[ "$scan_path" = /* ]] || scan_path="$PWD/$scan_path"
  report="$reports/$(echo "$lockfile" | tr / _).json"
  diagnostics="$report.stderr"
  # cargo-audit can return zero after a failed yank lookup. Never interpret
  # its otherwise-clean JSON as a complete scan when diagnostics report errors.
  # Explicit no-color overrides CI CARGO_TERM_COLOR=always, keeping the error
  # prefix machine-readable rather than hidden behind ANSI escape sequences.
  for attempt in 1 2; do
    scanner_status=0
    (cd "$reports" && "$audit" audit --file "$scan_path" --deny warnings --json --color never) \
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
# The scanner itself is an independently locked registry dependency graph.
tool_lock=".github/security-tools/cargo-audit-0.22.2.lock"
scan_lock "$tool_lock"
if ! python3 tools/rust-sources/audit.py check "$report" "$tool_lock" "$scanner_status"; then
  cat "$report"
  status=1
fi
# All product and build packages resolve from the single root lockfile.
scan_lock Cargo.lock
if ! python3 tools/rust-sources/audit.py check \
  "$report" Cargo.lock "$scanner_status"; then
  cat "$report"
  status=1
fi
# RustSec skips path packages. Match their exact original registry identities
# as well; all findings fail, including on locally modified or renamed code.
upstream_lock="$reports/upstream.Cargo.lock"
if python3 tools/rust-sources/audit.py build "$upstream_lock"; then
  echo 'Auditing vendored upstream identities'
  scan_lock "$upstream_lock"
  if ! python3 tools/rust-sources/audit.py check \
    "$report" "$upstream_lock" "$scanner_status"; then
    cat "$report"
    status=1
  fi
else
  status=1
fi
exit "$status"
