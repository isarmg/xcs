"""Exercise CI suite commands without a compiler, network or product data."""
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest


class SuiteCoverageTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="storage-suite-coverage-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        scripts = self.root / ".github/scripts"
        scripts.mkdir(parents=True)
        (self.root / "test/ci").mkdir(parents=True)
        for name in ("check.sh", "check-foundation.sh"):
            shutil.copy2((Path(__file__).resolve().parents[2] / ".github/scripts") / name, scripts)
        for name in ("release-package.sh", "release-publish.sh"):
            (self.root / "test/ci" / name).write_text("#!/bin/sh\nexit 0\n")
        binary_directory = self.root / "bin"
        binary_directory.mkdir()
        for name in ("cargo", "python3", "node"):
            command = binary_directory / name
            command.write_text(
                f"#!{sys.executable}\n"
                "import json, os, pathlib, sys\n"
                "with open(os.environ['SUITE_RECEIPT'], 'a') as output:\n"
                "    output.write(json.dumps({'tool': pathlib.Path(sys.argv[0]).name, "
                "'args': sys.argv[1:]}) + '\\n')\n"
            )
            command.chmod(0o700)
        self.receipt = self.root / "calls.jsonl"
        self.environment = dict(
            os.environ,
            PATH=f"{binary_directory}{os.pathsep}{os.environ['PATH']}",
            SUITE_RECEIPT=str(self.receipt),
        )

    def suite(self, name):
        self.receipt.unlink(missing_ok=True)
        subprocess.run(
            ["bash", str(self.root / ".github/scripts/check.sh"), name],
            cwd="/tmp", env=self.environment, capture_output=True, text=True, check=True,
        )
        return [json.loads(line) for line in self.receipt.read_text().splitlines()]

    def test_web_runs_unit_and_integration_tests(self):
        call, = self.suite("web")
        self.assertEqual(call["tool"], "cargo")
        self.assertEqual(call["args"][0], "test")
        self.assertIn("web-interface", call["args"])
        self.assertIn("--locked", call["args"])
        self.assertIn("--offline", call["args"])
        self.assertFalse({"--tests", "--lib", "--bins", "--no-run", "--doc"}.intersection(call["args"]))

    def test_build_tools_keep_log_filter_and_formatter_regressions(self):
        calls = self.suite("build-tools")
        self.assertIn({"tool": "cargo", "args": ["test", "--locked", "--offline", "--lib", "-p", "env_logger", "-p", "env_filter"]}, calls)
        self.assertTrue(any(call["tool"] == "cargo" and "web-bindings" in call["args"] for call in calls))

    def test_removed_demo_suite_fails_before_running_tools(self):
        result = subprocess.run(
            ["bash", str(self.root / ".github/scripts/check.sh"), "demo"],
            cwd="/tmp", env=self.environment, capture_output=True, text=True, check=False,
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("Usage: check.sh", result.stderr)
        self.assertFalse(self.receipt.exists())

    def test_source_gate_reparses_registered_target_closures(self):
        calls = self.suite("rust-sources")
        verification = [call for call in calls
                        if call["args"][:2] == ["tools/rust-sources/manage.py", "verify"]]
        self.assertEqual(len(verification), 1)
        self.assertIn("--metadata", verification[0]["args"])
        self.assertTrue(any("test_rust_diagnostics.py" in call["args"] for call in calls))

    def test_static_gate_requires_no_rust_or_metadata_resolution(self):
        calls = self.suite("static")
        self.assertTrue(all(call["tool"] in ("python3", "node") for call in calls))
        self.assertIn(
            {"tool": "python3", "args": [".github/scripts/check-workflows.py"]}, calls,
        )
        self.assertIn(
            {"tool": "python3", "args": ["tools/rust-sources/manage.py", "verify"]}, calls,
        )
        for directory in ("test/tools/rust-sources", "test/ci", "test/tools/perf-check"):
            self.assertIn(
                {"tool": "python3", "args": ["-m", "unittest", "discover", "-s",
                                           directory, "-p", "test_*.py"]}, calls,
            )
        self.assertEqual(calls[-1], {"tool": "node", "args": [".github/scripts/check-evidence-probes.mjs"]})
        self.assertFalse(any("--metadata" in call["args"] for call in calls))

    def test_build_tools_cover_report_parsing_without_running_a_workload(self):
        calls = self.suite("build-tools")
        tests = [call for call in calls
                 if call["tool"] == "cargo" and call["args"][0] == "test"
                 and "perf-check" in call["args"]]
        self.assertEqual(len(tests), 1)
        self.assertIn("perf-check", tests[0]["args"])
        self.assertTrue(any(
            call["tool"] == "python3" and "test/tools/perf-check" in call["args"]
            for call in calls
        ))
        self.assertFalse(any(
            call["tool"] == "cargo" and call["args"][0] == "run"
            and "perf-check" in call["args"]
            for call in calls
        ))

    def test_foundation_shards_match_explicit_command_inventory(self):
        inventory = json.loads((Path(__file__).with_name("foundation-commands.json")).read_text())
        for name, expected in inventory.items():
            with self.subTest(suite=name):
                self.assertEqual(self.suite(name), expected)

    def test_toml_gate_runs_once_in_language_and_aggregate_only(self):
        expected = {"tool": "python3", "args": [".github/scripts/check-toml-consolidation.py"]}
        inventory = json.loads((Path(__file__).with_name("foundation-commands.json")).read_text())
        self.assertEqual(self.suite("foundation-language").count(expected), 1)
        self.assertEqual(self.suite("foundation").count(expected), 1)
        for name in inventory:
            if name != "foundation-language":
                with self.subTest(suite=name):
                    self.assertNotIn(expected, self.suite(name))

    def test_pattern_gate_runs_once_in_language_and_aggregate_only(self):
        expected = {"tool": "python3", "args": [".github/scripts/check-pattern-matching.py"]}
        inventory = json.loads((Path(__file__).with_name("foundation-commands.json")).read_text())
        self.assertEqual(self.suite("foundation-language").count(expected), 1)
        self.assertEqual(self.suite("foundation").count(expected), 1)
        for name in inventory:
            if name != "foundation-language":
                with self.subTest(suite=name):
                    self.assertNotIn(expected, self.suite(name))

    def test_concurrent_collections_gate_runs_once_in_async_and_aggregate_only(self):
        expected = {"tool": "python3", "args": [".github/scripts/check-concurrent-collections.py"]}
        inventory = json.loads((Path(__file__).with_name("foundation-commands.json")).read_text())
        self.assertEqual(self.suite("foundation-async").count(expected), 1)
        self.assertEqual(self.suite("foundation").count(expected), 1)
        for name in inventory:
            if name != "foundation-async":
                with self.subTest(suite=name):
                    self.assertNotIn(expected, self.suite(name))

    def test_service_stack_gate_runs_once_in_async_and_aggregate_only(self):
        expected = {"tool": "python3", "args": [".github/scripts/check-service-stack.py"]}
        inventory = json.loads((Path(__file__).with_name("foundation-commands.json")).read_text())
        self.assertEqual(self.suite("foundation-async").count(expected), 1)
        self.assertEqual(self.suite("foundation").count(expected), 1)
        for name in inventory:
            if name != "foundation-async":
                with self.subTest(suite=name):
                    self.assertNotIn(expected, self.suite(name))

    def test_foundation_aggregate_is_exactly_once_union_of_shards(self):
        inventory = json.loads((Path(__file__).with_name("foundation-commands.json")).read_text())
        expected = [call for calls in inventory.values() for call in calls]
        self.assertEqual(self.suite("foundation"), expected)
        identities = Counter(json.dumps(call, sort_keys=True) for call in expected)
        self.assertTrue(identities)
        self.assertTrue(all(count == 1 for count in identities.values()))
        for call in expected:
            if call["tool"] == "cargo":
                self.assertEqual(call["args"][0], "test")
                self.assertTrue({"--locked", "--offline"}.issubset(call["args"]))
                self.assertFalse({"--workspace", "--all-features", "--all-targets"}.intersection(call["args"]))

    def test_unknown_foundation_shard_fails_before_running_tools(self):
        result = subprocess.run(
            ["bash", str(self.root / ".github/scripts/check-foundation.sh"), "foundation-missing"],
            cwd="/tmp", env=self.environment, capture_output=True, text=True, check=False,
        )
        self.assertEqual(result.returncode, 2)
        self.assertFalse(self.receipt.exists())


class DiagnosticGateCommands(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="storage-quality-commands-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        scripts = self.root / ".github/scripts"
        scripts.mkdir(parents=True)
        for name in ("quality.sh", "smb-sdk.sh", "check-rust-diagnostics.py"):
            shutil.copy2((Path(__file__).resolve().parents[2] / ".github/scripts") / name, scripts)
        self.binary_directory = self.root / "bin"
        self.binary_directory.mkdir()
        self.write_tool("node", "#!/bin/sh\nexit 0\n")
        self.write_tool("cargo", f'''#!{sys.executable}
import json, os, pathlib, sys
args = sys.argv[1:]
with open(os.environ['SUITE_RECEIPT'], 'a') as receipt:
    receipt.write(json.dumps(args) + '\\n')
if args[0] == 'metadata':
    root = os.environ['SUITE_ROOT']
    packages = [{{'id': name, 'name': name, 'manifest_path': root + '/' + path + '/Cargo.toml'}}
                for name, path in [('web-app', 'src/web'), ('smb-vfs', 'src/smb-vfs'),
                                   ('local-dependency', 'src/foundation/local')]]
    print(json.dumps({{'workspace_root': root, 'packages': packages,
                      'workspace_members': [p['id'] for p in packages]}}))
elif '--message-format=json' in args:
    stage = ('wasm' if '--target' in args else 'host') if args[0] == 'check' else args[0]
    if os.environ.get('SUITE_WARNING') == stage:
        print(json.dumps({{'reason': 'compiler-message',
                          'package_id': 'path+file:///src/foundation/local#1.0.0',
                          'message': {{'message': 'local dependency warning', 'level': 'warning',
                                      'rendered': 'warning: local dependency warning\\n'}}}}))
    print(json.dumps({{'reason': 'build-finished', 'success': True}}))
''')
        self.receipt = self.root / "calls.jsonl"
        self.environment = dict(
            os.environ,
            PATH=f"{self.binary_directory}{os.pathsep}{os.environ['PATH']}",
            SUITE_RECEIPT=str(self.receipt), SUITE_ROOT=str(self.root),
        )

    def write_tool(self, name, source):
        path = self.binary_directory / name
        path.write_text(source)
        path.chmod(0o700)

    def run_script(self, name, **environment):
        result = subprocess.run(
            ["bash", str(self.root / ".github/scripts" / name)], cwd="/tmp",
            env=dict(self.environment, **environment), capture_output=True, text=True,
            check=False,
        )
        calls = [json.loads(line) for line in self.receipt.read_text().splitlines()]
        return result, calls

    def test_quality_checks_first_party_clippy_and_host_and_wasm_dependency_warnings(self):
        result, calls = self.run_script("quality.sh")
        self.assertEqual(result.returncode, 0, result.stderr)
        metadata, formatting, clippy, host, wasm = calls
        self.assertEqual(metadata[0], "metadata")
        self.assertEqual(formatting, ["fmt", "--package", "web-app", "--package", "smb-vfs",
                                      "--", "--check"])
        self.assertEqual(clippy[0], "clippy")
        self.assertIn("--all-targets", clippy)
        self.assertIn("--message-format=json", clippy)
        self.assertEqual(clippy[-3:], ["--", "-D", "warnings"])
        self.assertNotIn("smb-vfs", clippy)
        self.assertIn("--workspace", host)
        self.assertIn("--all-targets", host)
        self.assertEqual(host[host.index("--exclude") + 1], "smb-vfs")
        self.assertEqual(wasm[wasm.index("--target") + 1], "wasm32-unknown-unknown")
        self.assertIn("--release", wasm)
        self.assertEqual([wasm[i + 1] for i, argument in enumerate(wasm) if argument == "-p"],
                         ["web-app"])
        for call in (clippy, host, wasm):
            self.assertTrue({"--locked", "--offline", "--message-format=json"}.issubset(call))

    def test_quality_rejects_foundation_warnings_in_each_compile_stage(self):
        for stage in ("clippy", "host", "wasm"):
            with self.subTest(stage=stage):
                result, _ = self.run_script("quality.sh", SUITE_WARNING=stage)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("1 Rust warning(s)", result.stderr)

    def prepare_sdk(self):
        cache = self.root / "target/smb-sdk"
        source = cache / "samba-4.23.0"
        source.mkdir(parents=True)
        (cache / "samba-4.23.0.tar.gz").write_bytes(b"fixture archive")
        recipe = hashlib.sha256((self.root / ".github/scripts/smb-sdk.sh").read_bytes()).hexdigest()
        (source / ".zxs-sdk-ready").write_text(recipe + "\n")
        self.write_tool("sha256sum", f'''#!{sys.executable}
import hashlib, pathlib, sys
if '--check' in sys.argv:
    sys.stdin.read()
else:
    print(hashlib.sha256(pathlib.Path(sys.argv[1]).read_bytes()).hexdigest(), sys.argv[1])
''')
        for name in ("cc", "pkg-config", "perl", "yapp", "flex", "bison", "m4"):
            self.write_tool(name, "#!/bin/sh\nexit 0\n")
        for name in ("curl", "tar"):
            self.write_tool(name, "#!/bin/sh\nexit 91\n")

    def test_sdk_gates_compile_streams_and_then_runs_normal_tests(self):
        self.prepare_sdk()
        result, calls = self.run_script("smb-sdk.sh")
        self.assertEqual(result.returncode, 0, result.stderr)
        clippy, compile_tests, tests, release = calls
        self.assertEqual(clippy[0], "clippy")
        self.assertIn("--all-targets", clippy)
        self.assertIn("--no-run", compile_tests)
        self.assertEqual(tests, ["test", "--locked", "--offline", "-p", "smb-vfs"])
        self.assertEqual(release[0], "build")
        self.assertIn("--release", release)
        for call in (clippy, compile_tests, release):
            self.assertTrue({"--locked", "--offline", "--message-format=json"}.issubset(call))

    def test_sdk_rejects_warnings_before_running_test_bodies(self):
        self.prepare_sdk()
        result, calls = self.run_script("smb-sdk.sh", SUITE_WARNING="test")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(len(calls), 2)
        self.assertIn("--no-run", calls[-1])
        self.assertIn("1 Rust warning(s)", result.stderr)


if __name__ == "__main__":
    unittest.main()
