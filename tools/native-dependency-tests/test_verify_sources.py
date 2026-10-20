import importlib.util
import pathlib
import shutil
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("verify", pathlib.Path(__file__).with_name("verify_sources.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

class SourceIntegrityTests(unittest.TestCase):
    def test_current_sources(self):
        module.verify()

    def test_tamper_and_missing_patch_fail_closed(self):
        for mode in ("source", "patch", "pointer", "symlink"):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as tmp:
                root = pathlib.Path(tmp)
                shutil.copytree(module.ROOT / "vendor/dependencies", root / "vendor/dependencies")
                (root / "apps/host").mkdir(parents=True)
                shutil.copy(module.ROOT / "apps/host/Cargo.toml", root / "apps/host/Cargo.toml")
                shutil.copy(module.ROOT / "apps/host/Cargo.lock", root / "apps/host/Cargo.lock")
                (root / "tools/native-dependency-tests").mkdir(parents=True)
                for filename in ("Cargo.toml", "Cargo.lock"):
                    shutil.copy(module.ROOT / "tools/native-dependency-tests" / filename, root / "tools/native-dependency-tests" / filename)
                if mode == "patch":
                    (root / "vendor/dependencies/patches/glib-0.18.5.patch").unlink()
                else:
                    source = root / "vendor/dependencies/glib-0.18.5/src/variant_iter.rs"
                    content = source.read_text()
                    if mode == "symlink":
                        external = root / "same-bytes-outside-maintained-tree.rs"
                        external.write_text(content)
                        source.unlink()
                        source.symlink_to(external)
                    else:
                        source.write_text(content + "// tampered\n" if mode == "source" else content.replace("&mut p,", "&p,"))
                with self.assertRaises((ValueError, FileNotFoundError)):
                    module.verify(root)

if __name__ == "__main__":
    unittest.main()
