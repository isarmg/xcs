"""A caller's encoded or explicit rustflags cannot collapse backend matrices."""
import importlib.util
import os
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

SCRIPTS = (Path(__file__).resolve().parents[2] / ".github/scripts")


def load(name):
    spec = importlib.util.spec_from_file_location(name.removesuffix(".py").replace("-", "_"), SCRIPTS / name)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


helper = load("backend_environment.py")


class BackendEnvironment(unittest.TestCase):
    def test_encoded_flags_are_rejected_even_when_empty(self):
        for value in ("", "-C\x1fdebuginfo=0", "--cfg\x1frustix_use_libc"):
            with self.subTest(value=value), patch.dict(os.environ, {"CARGO_ENCODED_RUSTFLAGS": value}, clear=True):
                with self.assertRaisesRegex(RuntimeError, "unset CARGO_ENCODED_RUSTFLAGS"):
                    helper.matrix_environment()

    def test_external_backend_or_optimization_flags_are_not_silently_discarded(self):
        for flags in ("--cfg rustix_use_libc", '--cfg system_entropy_backend="unsupported"',
                      "--cfg mio_unsupported_force_poll_poll", "-C opt-level=2", " "):
            with self.subTest(flags=flags), patch.dict(os.environ, {"RUSTFLAGS": flags}, clear=True):
                with self.assertRaisesRegex(RuntimeError, "unset RUSTFLAGS"):
                    helper.matrix_environment()

    def test_controlled_empty_flags_override_config_without_losing_other_environment(self):
        with patch.dict(os.environ, {"PATH": "/test/bin", "CARGO_BUILD_JOBS": "2", "RUSTFLAGS": "",
                                     "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS": "--cfg ignored"}, clear=True):
            environment = helper.matrix_environment()
        self.assertEqual(environment["RUSTFLAGS"], "")
        self.assertEqual(environment["PATH"], "/test/bin")
        self.assertEqual(environment["CARGO_BUILD_JOBS"], "2")
        self.assertNotIn("CARGO_ENCODED_RUSTFLAGS", environment)

    def test_every_flag_sensitive_entry_rejects_before_running_tools(self):
        for name in ("check-linux-system-backends.py", "check-linux-parking.py",
                     "check-linux-random-backends.py", "check-linux-syscalls.py"):
            checker = load(name)
            for environment in ({"CARGO_ENCODED_RUSTFLAGS": ""}, {"CARGO_ENCODED_RUSTFLAGS": "-C\x1fdebuginfo=0"},
                                {"RUSTFLAGS": "--cfg rustix_use_libc"}):
                with self.subTest(checker=name, environment=environment), patch.dict(os.environ, environment, clear=True), \
                        patch.object(checker.subprocess, "run") as run, \
                        patch.object(checker.subprocess, "Popen") as popen, patch.object(sys, "argv", [name]):
                    with self.assertRaisesRegex(RuntimeError, "unset (CARGO_ENCODED_RUSTFLAGS|RUSTFLAGS)"):
                        if name == "check-linux-random-backends.py":
                            checker.main()
                        else:
                            checker.check(Path("/nonexistent-backend-fixture"))
                    run.assert_not_called()
                    popen.assert_not_called()


if __name__ == "__main__":
    unittest.main()
