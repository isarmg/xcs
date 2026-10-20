"""Exercise root workspace helper builds without a registry or compiler."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


class LocalBuildHelper(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="zxs-offline-helper-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        scripts = self.root / ".github/scripts"
        scripts.mkdir(parents=True)
        for name in ("rust-build-tool.sh", "check-rust-diagnostics.py"):
            shutil.copy2(Path(__file__).parents[1] / name, scripts)
        self.script = scripts / "rust-build-tool.sh"
        self.source = self.root / "tools/web-bindings"
        self.source.mkdir(parents=True)
        (self.source / "Cargo.toml").write_text("[package]\n")
        (self.root / "Cargo.toml").write_text("[workspace]\n")
        (self.root / "Cargo.lock").write_text("version = 4\n")
        binary_directory = self.root / "bin"
        binary_directory.mkdir()
        cargo = binary_directory / "cargo"
        cargo.write_text('''#!/usr/bin/env python3
import json, os, pathlib, sys
args = sys.argv[1:]
with open(os.environ["ZXS_TEST_RECEIPT"], "a") as output:
    output.write(json.dumps({"args": args, "cwd": os.getcwd()}) + "\\n")
if os.environ.get("ZXS_TEST_FAIL"):
    sys.exit(17)
root = pathlib.Path(args[args.index("--target-dir") + 1])
binary = args[args.index("--bin") + 1]
tool = root / "release" / binary
tool.parent.mkdir(parents=True, exist_ok=True)
version = os.environ.get("ZXS_TEST_VERSION", "wasm-bindgen 0.2.129")
tool.write_text("#!/bin/sh\\nprintf '%s\\n' '" + version + "'\\n")
tool.chmod(0o700)
print(json.dumps({"reason": "build-finished", "success": True}))
''')
        cargo.chmod(0o700)
        self.receipt = self.root / "calls.jsonl"
        self.environment = dict(os.environ, PATH=f"{binary_directory}{os.pathsep}{os.environ['PATH']}",
                                ZXS_TEST_RECEIPT=str(self.receipt))

    def run_helper(self, helper="wasm-bindgen", **environment):
        return subprocess.run(["bash", str(self.script), helper], cwd="/tmp",
                              env=dict(self.environment, **environment), capture_output=True,
                              text=True, check=False)

    def calls(self):
        return [json.loads(line) for line in self.receipt.read_text().splitlines()]

    def test_empty_tool_build_uses_root_workspace_lock_offline(self):
        result = self.run_helper()
        self.assertEqual(result.returncode, 0, result.stderr)
        call, = self.calls()
        self.assertEqual(call["cwd"], str(self.root))
        self.assertIn("--locked", call["args"])
        self.assertIn("--offline", call["args"])
        self.assertEqual(call["args"][0], "build")
        self.assertEqual(call["args"][call["args"].index("--package") + 1], "web-bindings")
        self.assertNotIn("--manifest-path", call["args"])
        self.assertIn("--message-format=json", call["args"])
        self.assertEqual(Path(result.stdout.strip()).name, "wasm-bindgen")

    def test_maintained_source_edits_reenter_cargo_even_with_same_version(self):
        self.assertEqual(self.run_helper().returncode, 0)
        (self.source / "local-fix.rs").write_text("// maintained fix\n")
        self.assertEqual(self.run_helper().returncode, 0)
        self.assertEqual(len(self.calls()), 2)

    def test_missing_root_lock_fails_before_build(self):
        (self.root / "Cargo.lock").unlink()
        self.assertNotEqual(self.run_helper().returncode, 0)
        self.assertFalse(self.receipt.exists())

    def test_compiler_failure_does_not_return_cached_tool(self):
        self.assertEqual(self.run_helper().returncode, 0)
        result = self.run_helper(ZXS_TEST_FAIL="1")
        self.assertEqual(result.returncode, 17)
        self.assertEqual(result.stdout, "")

    def test_wrong_tool_version_fails(self):
        result = self.run_helper(ZXS_TEST_VERSION="wasm-bindgen 0.2.128")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "")
        self.assertIn("Expected wasm-bindgen 0.2.129", result.stderr)


if __name__ == "__main__":
    unittest.main()
