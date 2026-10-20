"""Fail closed on dependency warnings, malformed Cargo output and failed builds."""
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import unittest


SCRIPT = Path(__file__).parents[1] / "check-rust-diagnostics.py"
SPEC = importlib.util.spec_from_file_location("rust_diagnostics", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def diagnostic(level="warning", rendered="warning: local dependency warning\n"):
    return {"reason": "compiler-message", "package_id": "path+file:///source/foundation/local#1.0.0",
            "message": {"level": level, "message": "local dependency warning", "rendered": rendered}}


def finished(success=True):
    return {"reason": "build-finished", "success": success}


class DiagnosticStreamTests(unittest.TestCase):
    def check(self, events, returncode=0):
        output = io.StringIO()
        checker = MODULE.CargoDiagnostics(output)
        for number, event in enumerate(events, 1):
            checker.consume(json.dumps(event), number)
        return checker.result(returncode), output.getvalue()

    def test_success_with_artifacts_and_native_build_script_metadata(self):
        result, output = self.check([
            {"reason": "build-script-executed", "env": [["BINDGEN", "generated.rs"]]},
            {"reason": "compiler-artifact", "fresh": True}, finished(),
        ])
        self.assertEqual((result, output), (0, ""))

    def test_dependency_warning_rejects_even_when_cargo_succeeds(self):
        result, output = self.check([diagnostic(), finished()])
        self.assertNotEqual(result, 0)
        self.assertIn("warning: local dependency warning", output)
        self.assertIn("1 Rust warning(s)", output)

    def test_errors_reject_even_when_a_broken_producer_claims_success(self):
        for level in ("error", "failure-note", "ice"):
            with self.subTest(level=level):
                self.assertNotEqual(self.check([diagnostic(level), finished()])[0], 0)

    def test_notes_and_help_are_rendered_without_rejecting(self):
        for level in ("note", "help"):
            with self.subTest(level=level):
                result, output = self.check([diagnostic(level, f"{level}: context\n"), finished()])
                self.assertEqual((result, output), (0, f"{level}: context\n"))

    def test_diagnostic_without_rendered_text_remains_visible(self):
        result, output = self.check([diagnostic(rendered=None), finished()])
        self.assertNotEqual(result, 0)
        self.assertIn("warning: local dependency warning", output)

    def test_failed_or_missing_completion_cannot_pass(self):
        for events, code in (([], 0), ([finished(False)], 0), ([finished()], 101)):
            with self.subTest(events=events, code=code):
                self.assertNotEqual(self.check(events, code)[0], 0)

    def test_invalid_or_unknown_messages_cannot_pass(self):
        invalid = [None, [], {}, {"reason": "unexpected"},
                   {"reason": "compiler-message", "message": "not a diagnostic"},
                   diagnostic("unknown"), diagnostic(rendered=[]), finished("true")]
        for event in invalid:
            with self.subTest(event=event):
                events = [event]
                if not isinstance(event, dict) or event.get("reason") != "build-finished":
                    events.append(finished())
                self.assertNotEqual(self.check(events)[0], 0)

    def test_duplicate_completion_and_messages_after_completion_cannot_pass(self):
        for event in (finished(), {"reason": "compiler-artifact"}, diagnostic("note")):
            with self.subTest(event=event):
                self.assertNotEqual(self.check([finished(), event])[0], 0)

    def test_malformed_json_is_not_hidden_by_later_success(self):
        for text in ("", "warning: not Cargo JSON", '{"reason":', '{"reason": NaN}',
                     '{"reason":"compiler-message","reason":"compiler-artifact"}'):
            with self.subTest(text=text):
                output = io.StringIO()
                checker = MODULE.CargoDiagnostics(output)
                checker.consume(text, 1)
                checker.consume(json.dumps(finished()), 2)
                self.assertNotEqual(checker.result(0), 0)
                self.assertIn("invalid Cargo JSON", output.getvalue())


class DiagnosticCommandTests(unittest.TestCase):
    def command(self, stream, code=0):
        producer = ("import sys; sys.stdout.write(sys.argv[1]); "
                    "sys.stderr.write('Cargo progress\\n'); sys.exit(int(sys.argv[2]))")
        return subprocess.run([sys.executable, str(SCRIPT), "--", sys.executable,
                               "-c", producer, stream, str(code)],
                              capture_output=True, text=True, check=False)

    def test_real_child_exit_and_stderr_are_preserved(self):
        result = self.command(json.dumps(finished()) + "\n")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "")
        self.assertIn("Cargo progress", result.stderr)
        self.assertEqual(self.command(json.dumps(finished()) + "\n", 17).returncode, 17)

    def test_warning_stream_rejects_a_successful_child(self):
        result = self.command("\n".join(map(json.dumps, [diagnostic(), finished()])) + "\n")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("1 Rust warning(s)", result.stderr)

    def test_partial_stream_and_invalid_json_cannot_pass(self):
        for stream in ("", "not JSON\n", json.dumps(diagnostic("note")) + "\n"):
            with self.subTest(stream=stream):
                self.assertNotEqual(self.command(stream).returncode, 0)

    def test_missing_command_and_failed_launch_are_errors(self):
        for args in ([], ["--", "/nonexistent/storage-cargo"]):
            with self.subTest(args=args):
                result = subprocess.run([sys.executable, str(SCRIPT), *args],
                                        capture_output=True, text=True, check=False)
                self.assertNotEqual(result.returncode, 0)

    def test_command_arguments_and_compiler_environment_are_preserved(self):
        compiler_environment = {
            "RUSTFLAGS": "--cfg original_product_configuration",
            "CARGO_ENCODED_RUSTFLAGS": "--cfg\x1foriginal_encoded_configuration",
            "CARGO_TARGET_DIR": "/tmp/original-product-cache",
        }
        producer = (
            "import json, os, sys; "
            f"assert sys.argv[1:] == {['--locked', '--offline', '--message-format=json']!r}; "
            f"assert all(os.environ[k] == v for k, v in {compiler_environment!r}.items()); "
            "print(json.dumps({'reason': 'build-finished', 'success': True}))"
        )
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "--", sys.executable, "-c", producer,
             "--locked", "--offline", "--message-format=json"],
            capture_output=True, text=True, check=False,
            env=dict(os.environ, **compiler_environment),
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_invalid_utf8_cannot_pass(self):
        result = subprocess.run(
            [sys.executable, str(SCRIPT), "--", sys.executable, "-c",
             "import sys; sys.stdout.buffer.write(b'\\xff\\n')"],
            capture_output=True, check=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"not valid UTF-8", result.stderr)


if __name__ == "__main__":
    unittest.main()
