"""SQLite executable coverage stays wired to real tests and isolated Cargo locks."""
import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("sqlite_family", ROOT / ".github/scripts/check-sqlite-family.py")
module = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(module)


class SqliteFamily(unittest.TestCase):
    def test_foundation_gate_is_wired_after_root_lock_units(self):
        script = (ROOT / ".github/scripts/check.sh").read_text()
        self.assertIn("cargo test --locked --offline --lib -p sqlite-access\n    python3 .github/scripts/check-sqlite-family.py", script)

    def test_all_executable_commands_require_existing_offline_lock(self):
        for action in ("test", "check"):
            command = module.cargo_command(action, Path("Cargo.toml"), Path("target"), "--lib")
            self.assertEqual(command[:4], ["cargo", action, "--locked", "--offline"])
            self.assertIn("--manifest-path", command)
            self.assertIn("--target-dir", command)

    def test_captured_commands_override_forced_color(self):
        with patch.dict(os.environ, {"CARGO_TERM_COLOR": "always"}):
            command = module.cargo_command("check", Path("Cargo.toml"), Path("target"))
            self.assertEqual(command[command.index("--color") + 1], "never")
            self.assertEqual(os.environ["CARGO_TERM_COLOR"], "always")

    def test_negative_matches_colored_diagnostic_without_weakening_failure(self):
        diagnostic = "\x1b[91merror[E0603]\x1b[0m: private module"
        for status in (0, 1):
            with patch.object(module.subprocess, "run", return_value=subprocess.CompletedProcess([], status, "", diagnostic)):
                if status:
                    module.execute(["cargo"], ROOT, expected="error[E0603]")
                else:
                    with self.assertRaises(AssertionError):
                        module.execute(["cargo"], ROOT, expected="error[E0603]")

    def test_existing_integration_paths_are_not_replaced_by_fixtures(self):
        self.assertEqual(module.INTEGRATION_TESTS, {
            "hash_table_compatibility": "test/engine/integration/hash_table_compatibility.rs",
            "sqlite_foundation": "test/platform/integration/sqlite_foundation.rs",
        })
        self.assertTrue(all((ROOT / path).is_file() for path in module.INTEGRATION_TESTS.values()))

    def test_consumer_uses_actual_provider_paths_and_explicit_features(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = module.consumer_manifest(ROOT, Path(temporary), [])
            manifest = tomllib.loads(path.read_text())
            dependencies = manifest["dependencies"]
            self.assertEqual(dependencies["sqlite-access"]["features"], [])
            self.assertFalse(dependencies["sqlite-access"]["default-features"])
            for name in ("sqlite-access", "tokio", "hash-table", "hashlink"):
                self.assertEqual(Path(dependencies[name]["path"]), ROOT / "src/foundation" / name)
            self.assertIn("workspace", manifest)
            self.assertEqual({row["path"] for row in manifest["test"]},
                             {str(ROOT / path) for path in module.INTEGRATION_TESTS.values()})

    def test_disposable_package_keeps_tests_but_uses_real_dependency_providers(self):
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary)
            manifest = module.prepare_package(ROOT, work)
            parsed = tomllib.loads(manifest.read_text())
            self.assertEqual(parsed["lib"]["path"], "src/lib.rs")
            self.assertIn("workspace", parsed)
            for entry in parsed["dependencies"].values():
                if isinstance(entry, dict) and "path" in entry:
                    self.assertTrue(Path(entry["path"]).is_relative_to(ROOT / "src/foundation"))
            original = ROOT / "test/foundation/sqlite-access"
            for path in original.rglob("*"):
                if path.is_file():
                    self.assertEqual(path.read_bytes(), (work / path.relative_to(ROOT)).read_bytes())
            self.assertFalse((manifest.parent / "Cargo.lock").exists())

    def test_zero_or_missing_tests_cannot_claim_coverage(self):
        for text in ("", "test result: ok. 0 passed; 0 failed;", "test result: ok. 7 passed; 0 failed;"):
            with patch.object(module.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, text, "")):
                with self.assertRaises(AssertionError):
                    module.execute(["cargo"], ROOT, minimum_tests=8)

    def test_aggregate_preserved_tests_count_is_checked(self):
        output = "test result: ok. 6 passed; 0 failed;\ntest result: ok. 2 passed; 0 failed;\n"
        with patch.object(module.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, output, "")):
            module.execute(["cargo"], ROOT, minimum_tests=8)

    def test_negative_requires_failed_compilation_and_exact_diagnostic(self):
        for status, stderr in ((0, "error[E0603]"), (1, "unrelated failure")):
            with patch.object(module.subprocess, "run", return_value=subprocess.CompletedProcess([], status, "", stderr)):
                with self.assertRaises(AssertionError):
                    module.execute(["cargo"], ROOT, expected="error[E0603]")
        with patch.object(module.subprocess, "run", return_value=subprocess.CompletedProcess([], 1, "", "error[E0603]")):
            module.execute(["cargo"], ROOT, expected="error[E0603]")


if __name__ == "__main__":
    unittest.main()
