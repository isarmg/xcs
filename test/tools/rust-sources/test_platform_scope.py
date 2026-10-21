"""Repository-level regressions for the maintained Linux amd64/Web boundary."""
import importlib.util
from pathlib import Path
import re
import tomllib
import unittest

SPEC = importlib.util.spec_from_file_location("platform_source_manager", (Path(__file__).resolve().parents[3] / "tools/rust-sources" / "manage.py"))
manager = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(manager)
ROOT = manager.ROOT


class PlatformScope(unittest.TestCase):
    def test_product_profiles_have_only_the_supported_targets(self):
        policy = manager.read_json(ROOT / manager.POLICY)
        manager.verify_target_policy(policy)
        self.assertEqual(
            {(entry["id"], entry["target"]) for entry in manager.entries(policy)},
            {("root-native", manager.NATIVE_TARGET), ("web", manager.WEB_TARGET),
             ("smb", manager.NATIVE_TARGET), ("build-tools", manager.NATIVE_TARGET),
             ("validation-tools", manager.NATIVE_TARGET)},
        )

    def test_linux_uapi_preserves_every_amd64_feature_and_binding(self):
        directory = ROOT / "src/foundation/linux-raw-sys"
        manifest = tomllib.loads((directory / "Cargo.toml").read_text())
        source = (directory / "src/lib.rs").read_text()
        modules = re.findall(r'#\[cfg\(feature = "([^"]+)"\)\]\n(?:#\[cfg\([^\n]+\)\]\n)?#\[path = "([^"]+)"\]\npub mod ([^;]+);', source)
        self.assertEqual(len(modules), 23)
        self.assertEqual({path.split("/")[0] for _, path, _ in modules}, {"x86_64"})
        self.assertEqual(
            {feature for feature, _, _ in modules} | {"elf"},
            set(manifest["features"]) - {"default", "std", "no_std", "rustc-dep-of-std"},
        )
        self.assertEqual({path.name for path in (directory / "src").iterdir() if path.is_dir()}, {"x86_64"})
        # The local dispatch changes, but retained generated ABI definitions do not.
        catalog = manager.read_json(ROOT / manager.CATALOG)
        package = catalog["packages"]["src/foundation/linux-raw-sys"]
        origin = manager.origins(ROOT)[package["origin_directory"]]
        for _, relative, _ in modules:
            original = "src/" + relative
            self.assertEqual(manager.digest((directory / original).read_bytes()),
                             origin["original_files_sha256"][original])
        self.assertEqual(manager.digest((directory / "src/elf.rs").read_bytes()),
                         origin["original_files_sha256"]["src/elf.rs"])

    def test_msvc_discovery_is_removed_without_dropping_provenance(self):
        self.assertFalse((ROOT / "src/foundation/vcpkg").exists())
        for relative in ("src/foundation/libsqlite3-sys/Cargo.toml", "src/foundation/sqlx-sqlite/Cargo.toml", "src/foundation/libsqlite3-sys/build.rs"):
            self.assertNotIn("vcpkg", (ROOT / relative).read_text().lower())
        records = manager.origins(ROOT)
        retired = [item for item in records.values() if item["upstream"]["name"] == "vcpkg"]
        self.assertEqual(len(retired), 1)
        self.assertTrue(retired[0]["original_files_sha256"])
        self.assertTrue(retired[0]["central_license_materials"])
        for relative in retired[0]["central_license_materials"]:
            self.assertTrue((ROOT / relative).is_file())


if __name__ == "__main__":
    unittest.main()
