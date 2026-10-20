#!/usr/bin/env python3
"""Exercise the real shell scanner wrapper without network access."""
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "security.sh"


class ScannerDiagnosticsTests(unittest.TestCase):
    def run_scanner(self, behavior):
        script = SCRIPT.read_text()
        wrapper = script[script.index("scan_lock() {"):script.index("\nstatus=0\n")]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
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

    def test_clean_scan_runs_once(self):
        result, calls = self.run_scanner(":")
        self.assertIn("STATUS=0", result.stdout)
        self.assertEqual(calls, 1)


if __name__ == "__main__":
    unittest.main()
