"""Direct compiler harnesses use their source package, not the caller's Cargo context."""
import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]


def load(name, relative):
    spec = importlib.util.spec_from_file_location(name, ROOT / relative)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


abi = load("libc_abi_environment", "tools/rust-sources/check_libc_abi.py")
system = load("system_backend_environment", ".github/scripts/check-linux-system-backends.py")


class DirectRustcEnvironment(unittest.TestCase):
    def test_abi_compile_overrides_inherited_or_explicit_wrong_manifest(self):
        for inherited in ({}, {"CARGO_MANIFEST_DIR": "/wrong/inherited"}):
            for explicit in (None, {"CARGO_MANIFEST_DIR": "/wrong/explicit", "RUSTC_BOOTSTRAP": "1"}):
                with self.subTest(inherited=inherited, explicit=explicit), patch.dict(os.environ, inherited, clear=True):
                    original = None if explicit is None else explicit.copy()
                    with patch.object(abi.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, "", "")) as run:
                        abi.run(["rustc", "src/lib.rs"], env=explicit, manifest_dir=Path("/source/libc"))
                    self.assertEqual(run.call_args.kwargs["env"]["CARGO_MANIFEST_DIR"], "/source/libc")
                    self.assertEqual(explicit, original)
                    self.assertEqual(dict(os.environ), inherited)

    def test_system_probe_uses_current_serialization_feature(self):
        import tomllib

        class ManifestCaptured(Exception):
            pass

        def capture_manifest(command, **kwargs):
            manifest = Path(command[command.index("--manifest-path") + 1])
            dependency = tomllib.loads(manifest.read_text())["dependencies"]["sysinfo"]
            features = dependency["features"]
            self.assertIn("serialization", features)
            self.assertNotIn("serde", features)
            declared = tomllib.loads((Path(dependency["path"]) / "Cargo.toml").read_text())["features"]
            self.assertTrue(set(features) <= set(declared))
            raise ManifestCaptured

        with patch.object(system, "check_target_contract"), patch.object(system.subprocess, "run", side_effect=capture_manifest):
            with self.assertRaises(ManifestCaptured):
                system.check(ROOT)

    def test_each_system_guard_probe_uses_its_package_directory(self):
        with tempfile.TemporaryDirectory() as temporary:
            work = Path(temporary)
            calls = []

            def compile_probe(command, **kwargs):
                calls.append(kwargs)
                cfg = {command[i + 1] for i, token in enumerate(command) if token == "--cfg"}
                crate = Path(kwargs["env"]["CARGO_MANIFEST_DIR"]).name
                native = {'probe_os="linux"', 'probe_arch="x86_64"', 'probe_pointer_width="64"'}
                web = {'probe_os="unknown"', 'probe_arch="wasm32"', 'probe_pointer_width="32"'}
                allowed = cfg == native or (crate == "sysinfo" and cfg == web)
                return subprocess.CompletedProcess(command, 0 if allowed else 1, "", "rejected")

            with patch.dict(os.environ, {"CARGO_MANIFEST_DIR": "/wrong/package"}):
                with patch.object(system.subprocess, "run", side_effect=compile_probe):
                    system.check_target_contract(ROOT, work)
            self.assertEqual(len(calls), 27)
            for index, crate in enumerate(("sysinfo", "socket2", "mio")):
                for call in calls[index * 9:(index + 1) * 9]:
                    self.assertEqual(call["env"]["CARGO_MANIFEST_DIR"], str(ROOT / "src/foundation" / crate))


if __name__ == "__main__":
    unittest.main()
