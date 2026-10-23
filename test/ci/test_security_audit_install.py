#!/usr/bin/env python3
"""Offline fail-closed tests for the independent security-tool installer."""
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import tarfile
import tempfile
import tomllib
import unittest
from unittest.mock import patch

SCRIPT = (Path(__file__).resolve().parents[2] / ".github/scripts") / "install-security-audit.py"
SPEC = importlib.util.spec_from_file_location("security_audit_install", SCRIPT)
INSTALL = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(INSTALL)
ORIGINAL = b'''version = 4
[[package]]
name = "rustls-webpki"
version = "0.103.13"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "old"
dependencies = ["ring"]
[[package]]
name = "ring"
version = "0.17.14"
'''
PATCHED = ORIGINAL.replace(b'0.103.13', b'0.103.16').replace(b'"old"', b'"fixed"')


class ReviewedRecipeTests(unittest.TestCase):
    def test_checked_in_snapshot_matches_recipe_and_update_provenance(self):
        recipe = json.loads((INSTALL.CONFIG / "cargo-audit.json").read_text())
        snapshot = (INSTALL.CONFIG / "cargo-audit-0.22.2.lock").read_bytes()
        INSTALL.verify(snapshot, recipe["patched_lock_sha256"], "reviewed tool lock")
        packages = tomllib.loads(snapshot.decode())["package"]
        for change in recipe["lock_changes"]:
            with self.subTest(change=change):
                self.assertNotEqual(change["before"], change["after"])
                if change["after"] is not None:
                    self.assertEqual(packages.count(change["after"]), 1)
                if change["before"] is not None:
                    self.assertNotIn(change["before"], packages)
        # The security fixes' recorded archive identities must describe the
        # actual installed packages, not only an unrelated documentation hash.
        for prefix, name in [("webpki", "rustls-webpki"), ("tokio", "tokio")]:
            with self.subTest(package=name):
                matches = [package for package in packages if package["name"] == name]
                self.assertEqual(len(matches), 1)
                package = matches[0]
                self.assertEqual(package["source"], INSTALL.REGISTRY)
                self.assertEqual(package["checksum"], recipe[f"{prefix}_archive_sha256"])
                self.assertEqual(recipe[f"{prefix}_archive_url"],
                                 f"https://static.crates.io/crates/{name}/"
                                 f"{name}-{package['version']}.crate")
                self.assertEqual(sum(change["after"] == package
                                     for change in recipe["lock_changes"]), 1)


class InstallTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.path = Path(self.temporary.name)
        self.config = self.path / "config"
        self.config.mkdir()
        self.root = self.path / "target"
        stream = io.BytesIO()
        with tarfile.open(fileobj=stream, mode="w:gz") as archive:
            member = tarfile.TarInfo("cargo-audit-0.22.2/Cargo.lock")
            member.size = len(ORIGINAL)
            archive.addfile(member, io.BytesIO(ORIGINAL))
        self.archive = stream.getvalue()
        self.recipe = {
            "archive_url": "https://static.crates.io/crates/cargo-audit/cargo-audit-0.22.2.crate",
            "archive_sha256": INSTALL.digest(self.archive),
            "original_lock_sha256": INSTALL.digest(ORIGINAL),
            "patched_lock_sha256": INSTALL.digest(PATCHED),
            "webpki_archive_sha256": "fixed",
            "lock_changes": [{"before": tomllib.loads(ORIGINAL.decode())["package"][0],
                              "after": tomllib.loads(PATCHED.decode())["package"][0]}],
        }
        self.save()

    def save(self, snapshot=PATCHED):
        (self.config / "cargo-audit.json").write_text(json.dumps(self.recipe))
        (self.config / "cargo-audit-0.22.2.lock").write_bytes(snapshot)

    def fetch(self, _url, target):
        target.write_bytes(self.archive)

    def build(self, args, **kwargs):
        self.assertIn("--locked", args)
        self.assertEqual(args[args.index("--jobs") + 1], "2")
        source = Path(args[args.index("--path") + 1])
        self.assertEqual((source / "Cargo.lock").read_bytes(), PATCHED)
        self.assertFalse(source.is_relative_to(INSTALL.ROOT))
        binary = self.root / "bin/cargo-audit"
        binary.parent.mkdir(parents=True, exist_ok=True)
        binary.write_bytes(b"mock hardened binary")
        return subprocess.CompletedProcess(args, 0)

    def test_only_expected_semantic_change(self):
        INSTALL.validate_lock(ORIGINAL, PATCHED, self.recipe)
        changed = PATCHED.replace(b'0.17.14', b'0.17.15')
        self.recipe["patched_lock_sha256"] = INSTALL.digest(changed)
        with self.assertRaisesRegex(ValueError, "more than"):
            INSTALL.validate_lock(ORIGINAL, changed, self.recipe)

    def test_explicit_obsolete_dependency_removal_is_checked(self):
        removed = b'version = 4\n[[package]]\nname = "ring"\nversion = "0.17.14"\n'
        self.recipe["patched_lock_sha256"] = INSTALL.digest(removed)
        self.recipe["lock_changes"] = [{"before": tomllib.loads(ORIGINAL.decode())["package"][0],
                                       "after": None}]
        INSTALL.validate_lock(ORIGINAL, removed, self.recipe)
        self.recipe["lock_changes"] = []
        with self.assertRaisesRegex(ValueError, "more than"):
            INSTALL.validate_lock(ORIGINAL, removed, self.recipe)

    def test_tampered_snapshot_fails_before_download(self):
        self.save(PATCHED + b"# tampered\n")
        with patch.object(INSTALL, "download") as download:
            with self.assertRaisesRegex(ValueError, "patched tool lock"):
                INSTALL.install(self.config, self.root)
            download.assert_not_called()

    def test_download_failure_never_builds(self):
        with patch.object(INSTALL, "download", side_effect=OSError("network failed")), \
                patch.object(INSTALL.subprocess, "run") as build:
            with self.assertRaises(OSError):
                INSTALL.install(self.config, self.root)
            build.assert_not_called()

    def test_archive_and_original_lock_checksum_fail_closed(self):
        for field in ["archive_sha256", "original_lock_sha256"]:
            with self.subTest(field=field):
                previous = self.recipe[field]
                self.recipe[field] = "wrong"
                self.save()
                with patch.object(INSTALL, "download", side_effect=self.fetch), \
                        patch.object(INSTALL.subprocess, "run") as build:
                    with self.assertRaises(ValueError):
                        INSTALL.install(self.config, self.root)
                    build.assert_not_called()
                self.recipe[field] = previous

    def test_lock_replacement_failure_never_builds(self):
        original_write = Path.write_bytes

        def reject_lock(path, data):
            if path.name == "Cargo.lock":
                raise OSError("lock replacement failed")
            return original_write(path, data)

        with patch.object(INSTALL, "download", side_effect=self.fetch), \
                patch.object(Path, "write_bytes", reject_lock), \
                patch.object(INSTALL.subprocess, "run") as build:
            with self.assertRaisesRegex(OSError, "replacement"):
                INSTALL.install(self.config, self.root)
            build.assert_not_called()

    def test_old_binary_wrong_recipe_and_modified_binary_are_not_reused(self):
        binary = self.root / "bin/cargo-audit"
        binary.parent.mkdir(parents=True)
        binary.write_bytes(b"old binary with same version")
        with patch.object(INSTALL, "download", side_effect=self.fetch) as download, \
                patch.object(INSTALL.subprocess, "run", side_effect=self.build) as build, \
                patch.object(INSTALL.subprocess, "check_output", return_value=INSTALL.VERSION + "\n"):
            INSTALL.install(self.config, self.root)
            self.assertEqual(build.call_count, 1)
            INSTALL.install(self.config, self.root)
            self.assertEqual(build.call_count, 1)
            self.recipe["review_note"] = "new recipe"
            self.save()
            INSTALL.install(self.config, self.root)
            self.assertEqual(build.call_count, 2)
            binary.write_bytes(b"tampered binary")
            INSTALL.install(self.config, self.root)
            self.assertEqual(build.call_count, 3)
            self.assertEqual(download.call_count, 3)

    def test_malformed_cache_receipt_is_not_reused(self):
        stamp = self.path / "receipt.json"
        for value in [[], None, "invalid", {}]:
            with self.subTest(value=value):
                stamp.write_text(json.dumps(value))
                self.assertFalse(INSTALL.cached(self.path / "missing", stamp, "recipe"))

    def test_archive_path_and_link_escapes_fail_closed(self):
        for kind in ["parent", "symlink", "hardlink"]:
            with self.subTest(kind=kind):
                stream = io.BytesIO()
                with tarfile.open(fileobj=stream, mode="w:gz") as archive:
                    member = tarfile.TarInfo("../escape" if kind == "parent" else "cargo-audit-0.22.2/escape")
                    if kind != "parent":
                        member.type = tarfile.SYMTYPE if kind == "symlink" else tarfile.LNKTYPE
                        member.linkname = str(self.path / "escape")
                    archive.addfile(member, io.BytesIO())
                self.archive = stream.getvalue()
                self.recipe["archive_sha256"] = INSTALL.digest(self.archive)
                self.save()
                with patch.object(INSTALL, "download", side_effect=self.fetch), \
                        patch.object(INSTALL.subprocess, "run") as build:
                    with self.assertRaises(tarfile.TarError):
                        INSTALL.install(self.config, self.root)
                    build.assert_not_called()
                self.assertFalse((self.path / "escape").exists())

    def test_wrong_binary_version_is_not_reused(self):
        binary = self.root / "bin/cargo-audit"
        binary.parent.mkdir(parents=True)
        binary.write_bytes(b"binary")
        stamp = self.root / "verified-build.json"
        stamp.write_text(json.dumps({"recipe_sha256": "recipe",
                                     "binary_sha256": INSTALL.digest(binary.read_bytes())}))
        with patch.object(INSTALL.subprocess, "check_output", return_value="cargo-audit 0.22.1"):
            self.assertFalse(INSTALL.cached(binary, stamp, "recipe"))

    def test_unexpected_dependency_source_is_rejected(self):
        self.recipe["lock_changes"][0]["after"]["source"] = "git+https://example.invalid/source"
        with self.assertRaisesRegex(ValueError, "source"):
            INSTALL.validate_lock(ORIGINAL, PATCHED, self.recipe)

    def test_build_cannot_rewrite_reviewed_lock(self):
        def rewrite(args, **kwargs):
            self.build(args, **kwargs)
            source = Path(args[args.index("--path") + 1])
            (source / "Cargo.lock").write_bytes(ORIGINAL)

        with patch.object(INSTALL, "download", side_effect=self.fetch), \
                patch.object(INSTALL.subprocess, "run", side_effect=rewrite):
            with self.assertRaisesRegex(ValueError, "built tool lock"):
                INSTALL.install(self.config, self.root)
            self.assertFalse((self.root / "verified-build.json").exists())

    def test_failed_build_does_not_stamp(self):
        with patch.object(INSTALL, "download", side_effect=self.fetch), \
                patch.object(INSTALL.subprocess, "run", side_effect=subprocess.CalledProcessError(1, "cargo")):
            with self.assertRaises(subprocess.CalledProcessError):
                INSTALL.install(self.config, self.root)
            self.assertFalse((self.root / "verified-build.json").exists())


if __name__ == "__main__":
    unittest.main()
