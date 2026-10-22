"""Static checks for optional tools, not browser or WASM execution evidence."""
from pathlib import Path
import re
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]
DOWNLOAD = ROOT / "test/tools/browser-smoke/downloads/browser-probe.mjs"
SIMD = ROOT / "test/tools/web-blake3-simd"


class ManualWebToolTests(unittest.TestCase):
    def test_javascript_syntax_without_execution(self):
        for script in (DOWNLOAD, SIMD / "check.mjs"):
            with self.subTest(script=script):
                subprocess.run(["node", "--check", str(script)], check=True, capture_output=True, text=True)

    def test_download_helper_uses_existing_runner_fixture(self):
        source = DOWNLOAD.read_text()
        self.assertIn("export async function verifyDownloads(page)", source)
        fixtures = re.findall(r"new URL\('([^']+)', import.meta.url\)", source)
        self.assertEqual(len(fixtures), 1)
        fixture = (DOWNLOAD.parent / fixtures[0]).resolve()
        self.assertEqual(fixture, ROOT / "test/tools/browser-smoke/runner/download_faults.js")
        self.assertTrue(fixture.is_file())

    def test_manual_simd_runner_paths_and_input_contract(self):
        runner = SIMD / "run-check.sh"
        subprocess.run(["bash", "-n", str(runner)], check=True, capture_output=True, text=True)
        source = runner.read_text()
        self.assertIn('${1:-$HERE/../../..}', source)
        self.assertEqual((SIMD / "../../..").resolve(), ROOT)
        for relative in re.findall(r'"\$HERE/([^"\n]+)"', source):
            self.assertTrue((SIMD / relative).exists(), relative)
        self.assertTrue((SIMD / "fixture/src/main.rs").is_file())
        self.assertTrue((SIMD / "fixture/src/lib.rs").is_file())
        checker = (SIMD / "check.mjs").read_text()
        self.assertIn("const [simdPath, portablePath, referencePath] = process.argv.slice(2)", checker)
        self.assertIn("timingClaim: 'none'", checker)
        self.assertIn("browserValidation: 'not performed'", checker)
