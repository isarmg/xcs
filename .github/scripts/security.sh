#!/usr/bin/env bash
# Pin the scanner, but fetch current RustSec advisories on every run.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
audit_root="$PWD/target/security-tools"
audit="$audit_root/bin/cargo-audit"
if [[ ! -x "$audit" ]] || [[ "$("$audit" --version)" != 'cargo-audit 0.22.2' ]]; then
  # The online advisory scanner is development tooling, not a business build
  # dependency. Keep its existing registry installation outside this project's
  # offline source replacement configuration.
  (
    install_context=$(mktemp -d)
    trap 'rmdir "$install_context"' EXIT
    cd "$install_context"
    cargo install cargo-audit --version '=0.22.2' --locked --root "$audit_root" --force
  )
fi
python3 .github/scripts/tests/test_security_gate.py
python3 -m unittest discover -s tools/rust-sources -p 'test_audit.py'
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
