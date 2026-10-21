#!/usr/bin/env python3
"""Exercise the real shell scanner wrapper without network access."""
from pathlib import Path
import os
import subprocess
import tempfile
import tomllib
import unittest

SCRIPT = (Path(__file__).resolve().parents[2] / ".github/scripts") / "security.sh"


class ScannerDiagnosticsTests(unittest.TestCase):
    def run_scanner(self, behavior):
        script = SCRIPT.read_text()
        wrapper = script[script.index("scan_lock() {"):script.index("\nstatus=0\n")]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / ".cargo").mkdir()
            (root / ".cargo/audit.toml").write_bytes(
                (SCRIPT.parents[1] / "security-tools/audit.toml").read_bytes())
            home = root / "cargo-home"
            home.mkdir()
            (home / "audit.toml").write_text(
                "[database]\nfetch = false\n[yanked]\nenabled = false\n")
            fake = root / "fake-audit"
            fake.write_text("#!/usr/bin/env bash\nset -eu\n"
                            f'count_file="{root / "count"}"\n'
                            'count=$(cat "$count_file" 2>/dev/null || echo 0)\n'
                            'count=$((count + 1)); echo "$count" > "$count_file"\n'
                            + behavior + '\necho "{}"\n')
            fake.chmod(0o755)
            result = subprocess.run(
                ["bash", "-c", f'set -euo pipefail\nreports="{root}"\naudit="{fake}"\n'
                 + wrapper + '\nscan_lock Cargo.lock\necho "STATUS=$scanner_status"\n'],
                text=True, capture_output=True, check=True,
                env={**os.environ, "CARGO_TERM_COLOR": "always", "CARGO_HOME": str(home)},
            )
            return result, int((root / "count").read_text())

    def test_zero_exit_with_incomplete_yank_lookup_fails_after_one_retry(self):
        result, calls = self.run_scanner('echo "error: couldn\'t check if package is yanked: timeout" >&2')
        self.assertIn("STATUS=2", result.stdout)
        self.assertEqual(calls, 2)

    def test_transient_registry_error_can_recover_once(self):
        result, calls = self.run_scanner('if [[ "$count" -eq 1 ]]; then echo "error: registry timeout" >&2; fi')
        self.assertIn("STATUS=0", result.stdout)
        self.assertEqual(calls, 2)

    def test_isolated_policy_enables_all_live_checks(self):
        policy = tomllib.loads((SCRIPT.parents[1] / "security-tools/audit.toml").read_text())
        self.assertEqual(policy["advisories"]["ignore"], [])
        self.assertNotIn("severity_threshold", policy["advisories"])
        self.assertEqual(set(policy["advisories"]["informational_warnings"]),
                         {"unmaintained", "unsound", "notice"})
        self.assertEqual(policy["database"], {"url": "https://github.com/RustSec/advisory-db",
                                              "fetch": True, "stale": False})
        self.assertEqual(policy["yanked"], {"enabled": True, "update_index": True})
        result, calls = self.run_scanner(
            '[[ -f "$PWD/.cargo/audit.toml" ]] || exit 2; '
            '[[ "$*" == *"--file /"* ]] || exit 2')
        self.assertIn("STATUS=0", result.stdout)
        self.assertEqual(calls, 1)

    def test_permissive_home_policy_does_not_replace_isolated_policy(self):
        # Match the official scanner's cwd-first, then Cargo-home selection.
        result, calls = self.run_scanner(
            'config="$CARGO_HOME/audit.toml"; '
            '[[ ! -f .cargo/audit.toml ]] || config=.cargo/audit.toml; '
            'grep -q "fetch = true" "$config" || exit 2; '
            'grep -q "enabled = true" "$config" || exit 2; '
            'grep -q "update_index = true" "$config" || exit 2')
        self.assertIn("STATUS=0", result.stdout)
        self.assertEqual(calls, 1)

    def test_ci_color_setting_cannot_hide_incomplete_scan(self):
        result, calls = self.run_scanner(
            'if [[ "$CARGO_TERM_COLOR" == always && "$*" != *"--color never"* ]]; then '
            'printf "\\033[31merror: registry timeout\\033[0m\\n" >&2; '
            'else echo "error: registry timeout" >&2; fi')
        self.assertIn("STATUS=2", result.stdout)
        self.assertEqual(calls, 2)

    def test_clean_scan_runs_once(self):
        result, calls = self.run_scanner(":")
        self.assertIn("STATUS=0", result.stdout)
        self.assertEqual(calls, 1)


if __name__ == "__main__":
    unittest.main()
