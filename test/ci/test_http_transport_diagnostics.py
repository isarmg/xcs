"""Timeouts must retain captured Cargo lock/build/test diagnostics and still fail."""
from contextlib import redirect_stderr
import importlib.util
import io
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("http_transport_gate", ROOT / ".github/scripts/check-http-transport.py")
GATE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GATE)


class HttpTransportDiagnostics(unittest.TestCase):
    def test_timeouts_preserve_bytes_text_or_missing_output_without_retrying(self):
        for output, diagnostics in (
            (b"running 18 tests\n", b"Blocking waiting for file lock on build directory\ninvalid: \xff"),
            ("running 18 tests\n", "Compiling retained primitives\n"),
            (None, None),
        ):
            with self.subTest(output=output):
                failure = subprocess.TimeoutExpired(["cargo", "test"], 600, output=output, stderr=diagnostics)
                captured = io.StringIO()
                with patch.object(GATE.subprocess, "run", side_effect=failure) as run, redirect_stderr(captured):
                    with self.assertRaises(subprocess.TimeoutExpired) as raised:
                        GATE.run(Path("fixture"), "test", "--locked", "--lib")
                self.assertIs(raised.exception, failure)
                run.assert_called_once()
                self.assertEqual(run.call_args.kwargs["timeout"], 600)
                self.assertEqual(run.call_args.args[0], ["cargo", "test", "--locked", "--lib", "--color", "never", "--offline"])
                text = captured.getvalue()
                self.assertIn("timed out after 600s in fixture", text)
                for value in (output, diagnostics):
                    if value:
                        self.assertIn(value.decode("utf-8", errors="replace") if isinstance(value, bytes) else value, text)


if __name__ == "__main__":
    unittest.main()
