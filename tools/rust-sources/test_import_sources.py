"""Pinned archive re-import never writes into maintained sources or trusts tar paths."""
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("zxs_source_importer", Path(__file__).with_name("import_sources.py"))
importer = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(importer)


class SourceImport(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="zxs-source-import-test-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        (self.root / "vendor").mkdir()
        self.cache = self.root / "cache"
        self.cache.mkdir()
        self.output = self.root / "review"
        self.catalog = self.root / "vendor/rust-sources.json"
        self.payloads = {
            "Cargo.toml": b'[package]\nname = "mini"\nversion = "1.0.0"\n',
            "src/lib.rs": b"pub fn value() -> u8 { 7 }\n",
            "LICENSE": b"fixture license material\n",
            "tests/\u00e9.rs": b"// unicode filename receipt\n",
        }
        self.write_fixture()

    def write_fixture(self, special=()):
        archive_path = self.cache / "mini-1.0.0.crate"
        with tarfile.open(archive_path, "w:gz") as archive:
            for relative, data in self.payloads.items():
                member = tarfile.TarInfo(f"mini-1.0.0/{relative}")
                member.size = len(data)
                member.mode = 0o644
                archive.addfile(member, io.BytesIO(data))
            for member, data in special:
                archive.addfile(member, io.BytesIO(data) if data else None)
        archive_hash = importer.digest_file(archive_path)
        files = {p: hashlib.sha256(data).hexdigest() for p, data in self.payloads.items()}
        receipt = importer.checksum_bytes(files, archive_hash)
        files[".cargo-checksum.json"] = hashlib.sha256(receipt).hexdigest()
        self.package = {
            "directory": "vendor/rust/mini-1.0.0", "name": "mini", "version": "1.0.0",
            "files_sha256": files,
            "upstream": {
                "name": "mini", "version": "1.0.0", "archive_sha256": archive_hash,
                "archive_url": "https://static.crates.io/crates/mini/mini-1.0.0.crate",
            },
        }
        # Maintained forks are deliberately excluded, even with no archive data.
        self.inventory = {"packages": {
            self.package["directory"]: self.package,
            "vendor/dependencies/maintained-fork": {"changes": "must not be re-imported"},
        }}
        self.save_catalog()

    def save_catalog(self):
        self.catalog.write_text(json.dumps(self.inventory))

    def restore(self, **kwargs):
        return importer.restore(self.catalog, self.output, self.cache, **kwargs)

    def test_cached_archive_reconstructs_all_files_and_cargo_receipt(self):
        self.assertEqual(self.restore(), 1)
        restored = self.output / "mini-1.0.0"
        actual = {p.relative_to(restored).as_posix(): importer.digest_file(p)
                  for p in restored.rglob("*") if p.is_file()}
        self.assertEqual(actual, self.package["files_sha256"])
        self.assertEqual((restored / "tests/\u00e9.rs").read_bytes(), self.payloads["tests/\u00e9.rs"])
        self.assertFalse((self.root / "vendor/rust").exists())

    def test_existing_empty_output_is_supported(self):
        self.output.mkdir()
        self.assertEqual(self.restore(), 1)

    def test_nonempty_output_and_maintained_vendor_output_are_rejected(self):
        self.output.mkdir()
        marker = self.output / "owned"
        marker.write_text("keep")
        with self.assertRaisesRegex(ValueError, "new or empty"):
            self.restore()
        self.assertEqual(marker.read_text(), "keep")
        with self.assertRaisesRegex(ValueError, "maintained.*vendor"):
            importer.restore(self.catalog, self.root / "vendor/new-tree", self.cache)

    def test_output_and_parent_symlinks_are_rejected(self):
        target = self.root / "target"
        target.mkdir()
        self.output.symlink_to(target, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, "symlink"):
            self.restore()
        with self.assertRaisesRegex(ValueError, "parent symlink"):
            importer.restore(self.catalog, self.output / "child", self.cache)

    def test_missing_cache_never_fetches_implicitly(self):
        (self.cache / "mini-1.0.0.crate").unlink()
        with patch.object(importer, "urlopen") as fetch:
            with self.assertRaisesRegex(ValueError, "absent from cache"):
                self.restore()
            fetch.assert_not_called()
        self.assertFalse(self.output.exists())

    def test_corrupt_archive_checksum_is_rejected_before_tar_processing(self):
        (self.cache / "mini-1.0.0.crate").write_bytes(b"corrupt archive")
        with patch.object(importer.tarfile, "open") as unpack:
            with self.assertRaisesRegex(ValueError, "archive checksum mismatch"):
                self.restore()
            unpack.assert_not_called()
        self.assertFalse(self.output.exists())

    def test_archive_paths_cannot_escape_or_have_alternate_spellings(self):
        for name in ["mini-1.0.0/../escape", "/escape", "other-1.0.0/file",
                     "mini-1.0.0/src\\escape", "mini-1.0.0/src/./escape",
                     "mini-1.0.0/src//escape", "mini-1.0.0/C:/escape"]:
            with self.subTest(name=name):
                member = tarfile.TarInfo(name)
                member.size = 1
                self.write_fixture([(member, b"x")])
                with self.assertRaisesRegex(ValueError, "unsafe archive path"):
                    self.restore()
                self.assertFalse(self.output.exists())

    def test_links_devices_and_named_pipes_are_rejected(self):
        for kind in (tarfile.SYMTYPE, tarfile.LNKTYPE, tarfile.FIFOTYPE, tarfile.CHRTYPE,
                     tarfile.CONTTYPE):
            with self.subTest(kind=kind):
                member = tarfile.TarInfo("mini-1.0.0/alias")
                member.type = kind
                member.linkname = "../../outside"
                self.write_fixture([(member, b"")])
                with self.assertRaisesRegex(ValueError, "link or special"):
                    self.restore()
                self.assertFalse(self.output.exists())

    def test_duplicate_paths_are_rejected(self):
        member = tarfile.TarInfo("mini-1.0.0/src/lib.rs")
        member.size = 1
        self.write_fixture([(member, b"x")])
        with self.assertRaisesRegex(ValueError, "duplicate archive path"):
            self.restore()

    def test_only_pinned_cargo_vcs_metadata_omissions_are_reproduced(self):
        member = tarfile.TarInfo("mini-1.0.0/.gitignore")
        member.size = 1
        self.write_fixture([(member, b"x")])
        self.assertEqual(self.restore(), 1)
        self.assertFalse((self.output / "mini-1.0.0/.gitignore").exists())

    def test_unlisted_upstream_runtime_material_cannot_be_silently_dropped(self):
        member = tarfile.TarInfo("mini-1.0.0/src/unlisted.rs")
        member.size = 1
        self.write_fixture([(member, b"x")])
        with self.assertRaisesRegex(ValueError, "file inventory mismatch"):
            self.restore()

    def test_archive_file_drift_and_missing_files_are_rejected(self):
        self.package["files_sha256"]["src/lib.rs"] = "0" * 64
        self.save_catalog()
        with self.assertRaisesRegex(ValueError, "file checksum mismatch"):
            self.restore()
        self.package["files_sha256"]["absent.rs"] = "0" * 64
        self.save_catalog()
        with self.assertRaisesRegex(ValueError, "file inventory mismatch"):
            self.restore()
        self.assertFalse(self.output.exists())

    def test_manifest_identity_and_generated_receipt_are_verified(self):
        self.payloads["Cargo.toml"] = b'[package]\nname = "someone-else"\nversion = "1.0.0"\n'
        self.write_fixture()
        with self.assertRaisesRegex(ValueError, "manifest identity"):
            self.restore()
        self.setUp_original_manifest()
        self.package["files_sha256"][".cargo-checksum.json"] = "0" * 64
        self.save_catalog()
        with self.assertRaisesRegex(ValueError, "Cargo checksum receipt"):
            self.restore()

    def setUp_original_manifest(self):
        self.payloads["Cargo.toml"] = b'[package]\nname = "mini"\nversion = "1.0.0"\n'
        self.write_fixture()

    def test_unexpected_inventory_directory_or_archive_url_is_rejected(self):
        self.package["directory"] = "vendor/rust/../../elsewhere"
        self.save_catalog()
        with self.assertRaisesRegex(ValueError, "unexpected registry directory"):
            self.restore()
        self.package["directory"] = "vendor/rust/mini-1.0.0"
        self.package["upstream"]["archive_url"] = "http://127.0.0.1/private"
        self.save_catalog()
        with self.assertRaisesRegex(ValueError, "unexpected archive URL"):
            self.restore(fetch=True)

    def test_explicit_fetch_is_pinned_and_does_not_modify_cache(self):
        archive = (self.cache / "mini-1.0.0.crate").read_bytes()
        (self.cache / "mini-1.0.0.crate").unlink()

        class Response(io.BytesIO):
            def geturl(self):
                return "https://static.crates.io/crates/mini/mini-1.0.0.crate"

        with patch.object(importer, "urlopen", return_value=Response(archive)) as fetch:
            self.assertEqual(self.restore(fetch=True), 1)
            fetch.assert_called_once()
        self.assertEqual(list(self.cache.iterdir()), [])

    def test_redirected_or_corrupted_download_is_rejected(self):
        (self.cache / "mini-1.0.0.crate").unlink()

        class Redirected(io.BytesIO):
            def geturl(self):
                return "https://other.test/changed"

        with patch.object(importer, "urlopen", return_value=Redirected(b"wrong")):
            with self.assertRaisesRegex(ValueError, "redirected"):
                self.restore(fetch=True)
        self.assertFalse(self.output.exists())

        class Corrupted(io.BytesIO):
            def geturl(self):
                return "https://static.crates.io/crates/mini/mini-1.0.0.crate"

        with patch.object(importer, "urlopen", return_value=Corrupted(b"wrong")):
            with self.assertRaisesRegex(ValueError, "archive checksum mismatch"):
                self.restore(fetch=True)
        self.assertFalse(self.output.exists())

    def test_unknown_subset_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "unknown registry packages"):
            self.restore(selected=["unknown@1.0.0"])
        self.assertEqual(self.restore(selected=["mini@1.0.0"]), 1)

    def test_current_monorepo_catalog_cannot_silently_import_zero_packages(self):
        self.inventory = {"packages": {"src/foundation/mini": {"upstream": self.package["upstream"]}}}
        self.save_catalog()
        with self.assertRaisesRegex(ValueError, "no original registry receipts"):
            self.restore()

    def test_original_archive_import_cannot_write_anywhere_in_maintained_monorepo(self):
        from unittest.mock import patch
        with patch.object(importer, "ROOT", self.root):
            with self.assertRaisesRegex(ValueError, "maintained repository"):
                self.restore()


if __name__ == "__main__":
    unittest.main()
