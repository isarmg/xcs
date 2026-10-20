"""Keep the maintained syscall source boundary and upstream amd64 instructions."""
import importlib.util
from pathlib import Path
import tomllib
import unittest

SPEC = importlib.util.spec_from_file_location("syscall_source_manager", Path(__file__).with_name("manage.py"))
manager = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(manager)
ROOT = manager.ROOT


class SyscallScope(unittest.TestCase):
    def test_amd64_assembly_is_unchanged_and_other_dispatches_are_gone(self):
        directory = ROOT / "src/foundation/rustix"
        arch = directory / "src/backend/linux_raw/arch"
        self.assertEqual({path.name for path in arch.iterdir()}, {"mod.rs", "x86_64.rs"})
        dispatch = (arch / "mod.rs").read_text()
        self.assertIn('#[path = "x86_64.rs"]', dispatch)
        self.assertNotIn("target_arch", dispatch)
        package = manager.read_json(ROOT / manager.CATALOG)["packages"]["src/foundation/rustix"]
        origin = manager.origins(ROOT)[package["origin_directory"]]
        relative = "src/backend/linux_raw/arch/x86_64.rs"
        self.assertEqual(manager.digest((directory / relative).read_bytes()),
                         origin["original_files_sha256"][relative])
        self.assertTrue((directory / "src/backend/libc/mod.rs").is_file())

    def test_manifest_preserves_raw_and_libc_backends_inside_native_boundary(self):
        manifest = tomllib.loads((ROOT / "src/foundation/rustix/Cargo.toml").read_text())
        self.assertEqual(len(manifest["target"]), 2)
        for expression, table in manifest["target"].items():
            for required in ('target_os = "linux"', 'target_arch = "x86_64"', 'target_pointer_width = "64"'):
                self.assertIn(required, expression)
            self.assertNotIn("target_env", expression)
            self.assertEqual(set(table["dependencies"]), {"linux-raw-sys", "libc", "libc_errno"})
            optional = "not(rustix_use_libc)" in expression
            for name in ("libc", "libc_errno"):
                self.assertEqual(table["dependencies"][name].get("optional", False), optional)
        self.assertEqual(manifest["features"]["use-libc"], ["libc_errno", "libc"])
        self.assertIn("all-apis", manifest["features"])


if __name__ == "__main__":
    unittest.main()
