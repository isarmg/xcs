"""Keep HTTP body feature boundaries explicit after package consolidation."""
import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "http_body_gate", ROOT / ".github/scripts/check-http-body-consolidation.py"
)
GATE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GATE)


def metadata(names):
    return {
        "packages": [{"id": name, "name": name} for name in names],
        "resolve": {"nodes": [{"id": name} for name in names]},
    }


class HttpBodyDependencyTests(unittest.TestCase):
    def test_bare_body_keeps_minimal_dependencies(self):
        GATE.validate_dependencies(metadata(["http-body", "bytes", "http"]), ())

    def test_utilities_do_not_activate_runtime(self):
        names = ["http-body", "futures-core", "pin-project-lite"]
        GATE.validate_dependencies(metadata(names), ("utilities",))
        with self.assertRaisesRegex(AssertionError, "Tokio"):
            GATE.validate_dependencies(metadata(names + ["tokio"]), ("utilities",))

    def test_channel_requires_runtime(self):
        names = ["http-body", "futures-core", "pin-project-lite", "tokio"]
        GATE.validate_dependencies(metadata(names), ("channel",))
        with self.assertRaisesRegex(AssertionError, "Tokio"):
            GATE.validate_dependencies(metadata(names[:-1]), ("channel",))

    def test_old_package_and_eager_utility_dependencies_are_rejected(self):
        with self.assertRaisesRegex(AssertionError, "absorbed"):
            GATE.validate_dependencies(metadata(["http-body", "http-body-util"]), ())
        with self.assertRaisesRegex(AssertionError, "isolation"):
            GATE.validate_dependencies(metadata(["http-body", "futures-core"]), ())


if __name__ == "__main__":
    unittest.main()
