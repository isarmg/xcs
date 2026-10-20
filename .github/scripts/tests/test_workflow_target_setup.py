"""Keep cross-target checks paired with their runner prerequisites."""
from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[3]


class WorkflowTargetSetupTests(unittest.TestCase):
    def test_quality_and_foundation_install_wasm_before_running(self):
        workflow = (ROOT / ".github/workflows/ci.yml").read_text()
        matrix_job = workflow.split("\n  tests:\n", 1)[1].split(
            "\n  web-production:\n", 1
        )[0]
        setup = re.search(
            r"- name: Install WASM standard library[^\n]*\n"
            r"\s+if: ([^\n]+)\n"
            r"\s+run: rustup target add wasm32-unknown-unknown",
            matrix_job,
        )
        self.assertIsNotNone(setup, "matrix jobs need explicit WASM target setup")
        self.assertEqual(
            set(re.findall(r"matrix\.suite == '([^']+)'", setup[1])),
            {"quality", "foundation"},
        )
        self.assertEqual(
            setup[1].strip(),
            "matrix.suite == 'quality' || matrix.suite == 'foundation'",
        )
        self.assertLess(setup.end(), matrix_job.index("Run locked test suite"))


if __name__ == "__main__":
    unittest.main()
