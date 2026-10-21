"""Ensure all preserved UAPI features receive both ABI regression modes."""
import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("linux_uapi_check", (Path(__file__).resolve().parents[2] / ".github/scripts") / "check-linux-uapi.py")
checker = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(checker)


class LinuxUapiCheck(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="linux-uapi-check-test-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.directory = self.root / "src/foundation/linux-raw-sys"
        self.directory.mkdir(parents=True)
        self.manifest = self.directory / "Cargo.toml"
        self.manifest.write_text(
            '[package]\nname = "linux-raw-sys"\nedition = "2021"\n'
            '[features]\ndefault = ["std"]\nstd = []\nno_std = []\n'
            'rustc-dep-of-std = ["no_std"]\ngeneral = []\nfuture_api = []\n'
        )

    def test_every_api_is_checked_in_std_and_no_std_modes(self):
        with patch.object(checker.subprocess, "run") as run:
            checker.check(self.root)
        self.assertEqual(run.call_count, 4)
        for offset, mode in ((0, "std"), (2, "no_std")):
            build = run.call_args_list[offset]
            command = build.args[0]
            self.assertEqual(command[0], "rustc")
            self.assertEqual(command[command.index("--target") + 1], checker.TARGET)
            self.assertIn("--test", command)
            self.assertEqual(command[command.index("--deny") + 1], "warnings")
            enabled = [command[index + 1] for index, value in enumerate(command) if value == "--cfg"]
            self.assertEqual(set(enabled), {f'feature="{mode}"', 'feature="general"', 'feature="future_api"'})
            self.assertEqual(run.call_args_list[offset + 1].args[0], [command[command.index("-o") + 1]])
            self.assertEqual(build.kwargs["cwd"], self.root)
            self.assertTrue(build.kwargs["check"])
            self.assertEqual(build.kwargs["env"]["CARGO_MANIFEST_DIR"], str(self.directory))

    def test_manifest_directory_overrides_missing_or_unrelated_cargo_context(self):
        for inherited in ({}, {"CARGO_MANIFEST_DIR": "/unrelated/package"}):
            with self.subTest(inherited=inherited), patch.dict(os.environ, inherited, clear=True):
                with patch.object(checker.subprocess, "run") as run:
                    checker.check(self.root)
                for call in run.call_args_list:
                    self.assertEqual(call.kwargs["env"]["CARGO_MANIFEST_DIR"], str(self.directory))
                self.assertEqual(dict(os.environ), inherited)

    def test_new_dependencies_require_a_cargo_based_review(self):
        with self.manifest.open("a") as manifest:
            manifest.write('[dependencies]\nnew_dependency = { path = "../other" }\n')
        with patch.object(checker.subprocess, "run") as run:
            with self.assertRaisesRegex(ValueError, "dependency-free"):
                checker.check(self.root)
        run.assert_not_called()

    def test_compile_failure_never_runs_a_stale_executable(self):
        with patch.object(checker.subprocess, "run", side_effect=subprocess.CalledProcessError(1, "rustc")) as run:
            with self.assertRaises(subprocess.CalledProcessError):
                checker.check(self.root)
        self.assertEqual(run.call_count, 1)


if __name__ == "__main__":
    unittest.main()
