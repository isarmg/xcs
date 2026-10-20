"""The syscall regression must use both current Cargo artifacts, never a glob."""
import importlib.util
import json
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("linux_syscalls_check", Path(__file__).parents[1] / "check-linux-syscalls.py")
checker = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(checker)


def artifact(name):
    return json.dumps({
        "reason": "compiler-artifact", "target": {"name": name, "kind": ["lib"]},
        "filenames": [f"/tmp/deps/lib{name}-current.rlib"],
    })


class LinuxSyscallsCheck(unittest.TestCase):
    def test_both_backends_use_locked_consumer_and_exact_artifacts(self):
        output = "\n".join([artifact("rustix"), artifact("libc")])
        with patch.object(checker.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, output)) as run:
            checker.check(Path("/repo"))
        self.assertEqual(run.call_count, 8)
        guard = run.call_args_list[0].args[0]
        self.assertIn("/repo/src/foundation/rustix/build.rs", guard)
        self.assertEqual(run.call_args_list[1].args[0], [guard[guard.index("-o") + 1]])
        for offset, features in ((2, "rustix/all-apis"), (5, "rustix/all-apis,rustix/use-libc")):
            cargo = run.call_args_list[offset].args[0]
            self.assertEqual(cargo[:4], ["cargo", "build", "--locked", "--offline"])
            self.assertEqual(cargo[cargo.index("--features") + 1], features)
            self.assertEqual(cargo[cargo.index("--target") + 1], checker.TARGET)
            rustc = run.call_args_list[offset + 1].args[0]
            self.assertIn("rustix=/tmp/deps/librustix-current.rlib", rustc)
            self.assertIn("libc=/tmp/deps/liblibc-current.rlib", rustc)
            self.assertEqual(run.call_args_list[offset + 2].args[0], [rustc[rustc.index("-o") + 1]])

    def test_missing_or_ambiguous_artifacts_fail_closed(self):
        for output in ("", artifact("rustix"), "\n".join([artifact("rustix"), artifact("rustix"), artifact("libc")])):
            with self.assertRaises(ValueError):
                checker.libraries(output)

    def test_cargo_diagnostics_are_not_silently_discarded(self):
        output = json.dumps({"reason": "compiler-message", "message": {
            "level": "warning", "message": "new foundation diagnostic",
        }})
        with self.assertRaisesRegex(ValueError, "new foundation diagnostic"):
            checker.libraries(output)

    def test_build_failure_never_executes_stale_test_binary(self):
        with patch.object(checker.subprocess, "run", side_effect=subprocess.CalledProcessError(1, "cargo")) as run:
            with self.assertRaises(subprocess.CalledProcessError):
                checker.check(Path("/repo"))
        self.assertEqual(run.call_count, 1)


if __name__ == "__main__":
    unittest.main()
