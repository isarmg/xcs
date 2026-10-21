"""Current-source and bounded historical-archive regression guards for ring."""
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import shutil
import tempfile
import tomllib
import unittest
from unittest import mock
import warnings
import zipfile

SPEC = importlib.util.spec_from_file_location('ring_target_checker', (Path(__file__).resolve().parents[3] / "tools/rust-sources" / 'check_ring_tests.py'))
checker = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(checker)


class CurrentRingTarget(unittest.TestCase):
    def setUp(self):
        # Keep repeated source copies off small /tmp tmpfs, as with the matrix.
        parent = checker.ROOT / 'target/ring-target-proof-tests'
        parent.mkdir(parents=True, exist_ok=True)
        temporary = tempfile.TemporaryDirectory(dir=parent)
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        # This fixture exercises the frozen pre-routing pruning proof. Read its
        # exact bytes through authenticated routing, then isolate the registry.
        # Real current routing remains covered by live metadata and routing tests.
        view = checker.proof_routing.load(checker.ROOT)
        for base in (str(checker.RING), 'test/foundation/ring'):
            for relative in view.inventory(base):
                destination = self.root / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(view.read(relative))
        routing = mock.patch.object(checker.proof_routing, 'RECEIPTS', ())
        routing.start()
        self.addCleanup(routing.stop)
        (self.root / checker.RELOCATION).parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(checker.ROOT / checker.RELOCATION, self.root / checker.RELOCATION)
        self.record = json.loads((checker.ROOT / checker.TARGET).read_text())
        for relative in [checker.TARGET, checker.RETIRED_ARCHIVE, *self.record['historical_proofs']]:
            destination = self.root / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(checker.ROOT / relative, destination)

    def test_current_sources_and_historical_archive_verify(self):
        historical = checker.target_sources(self.root)
        self.assertEqual(len(historical), 120)
        self.assertEqual(set(historical), {
            str(checker.RING / row['path']) for row in self.record['deleted']
        } | {str(checker.RING / path) for path in ('build.rs', 'Cargo.toml', 'src/lib.rs', 'src/rand.rs')})
        self.assertEqual(len(self.record['retained']), 331)

    def test_relocation_receipt_is_authenticated(self):
        path = self.root / checker.RELOCATION
        path.write_bytes(path.read_bytes() + b' ')
        with self.assertRaisesRegex(ValueError, 'relocation receipt changed'):
            checker.target_sources(self.root)

    def test_external_missing_and_added_files_fail_before_history(self):
        path = self.root / 'test/foundation/ring/tests/hmac_tests.txt'
        original = path.read_bytes()
        path.unlink()
        with self.assertRaisesRegex(ValueError, 'current ring source inventory'):
            checker.target_sources(self.root)
        path.write_bytes(original)
        (path.parent / 'unaccounted.rs').write_text('unaccounted')
        with self.assertRaisesRegex(ValueError, 'current ring source inventory'):
            checker.target_sources(self.root)

    def test_external_parent_symlinks_are_rejected(self):
        path = self.root / 'test/foundation/ring/cavp'
        shutil.rmtree(path)
        path.symlink_to(checker.ROOT / 'test/foundation/ring/cavp', target_is_directory=True)
        with self.assertRaisesRegex(ValueError, 'symlink'):
            checker.target_sources(self.root)

    def test_all_three_historical_proofs_are_immutable(self):
        for relative in self.record['historical_proofs']:
            with self.subTest(path=relative):
                path = self.root / relative
                original = path.read_bytes()
                path.write_bytes(original + b' ')
                with self.assertRaisesRegex(ValueError, 'immutable historical proof'):
                    checker.target_sources(self.root)
                path.write_bytes(original)

    def test_fourth_proof_cannot_redefine_deleted_or_current_files(self):
        path = self.root / checker.TARGET
        record = self.record.copy()
        record['deleted'] = []
        path.write_text(json.dumps(record))
        with self.assertRaisesRegex(ValueError, 'immutable target-pruning proof'):
            checker.target_sources(self.root)

    def test_archive_cannot_supply_missing_active_source(self):
        (self.root / checker.RING / 'crypto/cpu_intel.c').unlink()
        with mock.patch.object(checker, 'read_retired_archive') as read:
            with self.assertRaisesRegex(ValueError, 'current ring source inventory'):
                checker.target_sources(self.root)
            read.assert_not_called()

    def test_active_source_and_fixture_edits_fail_before_archive_reads(self):
        for relative in ('crypto/cpu_intel.c', 'tests/hmac_tests.txt'):
            with self.subTest(path=relative):
                path = self.root / ('test/foundation/ring' if relative.startswith('tests/') else checker.RING) / relative
                original = path.read_bytes()
                path.write_bytes(original + b'bad')
                with mock.patch.object(checker, 'read_retired_archive') as read:
                    with self.assertRaisesRegex(ValueError, 'current ring source inventory/content'):
                        checker.target_sources(self.root)
                    read.assert_not_called()
                path.write_bytes(original)

    def test_unrecorded_build_change_fails(self):
        path = self.root / checker.RING / 'build.rs'
        path.write_bytes(path.read_bytes() + b'// unrecorded\n')
        with self.assertRaisesRegex(ValueError, 'current ring source inventory/content'):
            checker.target_sources(self.root)

    def test_retired_source_cannot_reappear(self):
        path = self.root / checker.RING / self.record['deleted'][0]['path']
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text('returned source')
        with self.assertRaisesRegex(ValueError, 'current ring source inventory'):
            checker.target_sources(self.root)

    def test_symlink_is_not_an_active_source(self):
        path = self.root / checker.RING / 'crypto/cpu_intel.c'
        path.unlink()
        path.symlink_to(checker.ROOT / checker.RING / 'crypto/cpu_intel.c')
        with self.assertRaisesRegex(ValueError, 'symlink'):
            checker.target_sources(self.root)

    def test_archive_size_and_hash_are_checked_before_zip_parse(self):
        path = self.root / checker.RETIRED_ARCHIVE
        original = path.read_bytes()
        for content in (original + b'x', b'x' + original[1:]):
            with self.subTest(length=len(content)), mock.patch.object(checker.zipfile, 'ZipFile') as parse:
                path.write_bytes(content)
                with self.assertRaises(ValueError):
                    checker.target_sources(self.root)
                parse.assert_not_called()

    def test_native_guard_limits_platform_not_libc_vendor(self):
        source = (self.root / checker.RING / 'build.rs').read_text()
        guard = source.split('fn main() {', 1)[1].split('// Avoid assuming', 1)[0]
        for name, value in [('ARCH', 'x86_64'), ('OS', 'linux'), ('POINTER_WIDTH', '64'), ('ENDIAN', 'little')]:
            self.assertIn(f'env::var("CARGO_CFG_TARGET_{name}").as_deref() == Some("{value}")', guard)
        self.assertNotIn('TARGET_ENV', guard)
        self.assertNotIn('TARGET_VENDOR', guard)

    def test_retired_features_and_target_dependencies_are_not_advertised(self):
        manifest = tomllib.loads((self.root / checker.RING / 'Cargo.toml').read_text())
        for feature in self.record['manifest_cleanup']['retired_features']:
            self.assertNotIn(feature, manifest['features'])
        self.assertEqual(set(manifest['target']), {'cfg(all(target_os = "linux", target_arch = "x86_64"))'})
        self.assertEqual(set(next(iter(manifest['target'].values()))), {'dev-dependencies'})

    def test_package_include_has_no_missing_entries(self):
        root = self.root / checker.RING
        manifest = tomllib.loads((root / 'Cargo.toml').read_text())
        for pattern in manifest['package']['include']:
            with self.subTest(pattern=pattern):
                self.assertTrue(any(path.is_file() for path in root.glob(pattern)))

    def test_metadata_reversal_preserves_native_random_expression(self):
        historical = checker.target_sources(self.root)
        before = historical[str(checker.RING / 'src/rand.rs')].decode()
        current = (self.root / checker.RING / 'src/rand.rs').read_text()
        marker = 'impl sealed::SecureRandom for SystemRandom'
        self.assertEqual(before.split(marker, 1)[1], current.split(marker, 1)[1])
        for patch in self.record['source_patches']:
            source = historical[str(checker.RING / patch['path'])]
            checker.check_content(source, patch['before'])


class RetiredArchiveBoundaries(unittest.TestCase):
    def archive(self, names=('retired.c',), mode=0o100644, compression=zipfile.ZIP_DEFLATED):
        buffer = io.BytesIO()
        with warnings.catch_warnings():
            warnings.simplefilter('ignore', UserWarning)
            with zipfile.ZipFile(buffer, 'w') as archive:
                for name in names:
                    info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
                    info.external_attr = mode << 16
                    info.compress_type = compression
                    archive.writestr(info, b'origin')
        content = buffer.getvalue()
        record = dict(path='fixture.zip', bytes=len(content), sha256=hashlib.sha256(content).hexdigest(),
                      members=len(names), uncompressed_bytes=6 * len(names))
        deleted = {name: dict(path=name, bytes=6, sha256=hashlib.sha256(b'origin').hexdigest(),
                             blob=checker.object_hash('blob', b'origin')) for name in names}
        return content, record, deleted

    def test_valid_regular_member_is_read_without_extraction(self):
        with mock.patch.object(zipfile.ZipFile, 'extractall') as extract:
            self.assertEqual(checker.read_retired_archive(*self.archive()), {'retired.c': b'origin'})
            extract.assert_not_called()

    def test_duplicate_member_is_rejected(self):
        with self.assertRaisesRegex(ValueError, 'inventory'):
            checker.read_retired_archive(*self.archive(('retired.c', 'retired.c')))

    def test_extra_or_missing_member_is_rejected(self):
        for extra in (True, False):
            content, record, deleted = self.archive()
            if extra:
                deleted.clear()
            else:
                deleted['another.c'] = deleted['retired.c']
            with self.subTest(extra=extra), self.assertRaisesRegex(ValueError, 'inventory'):
                checker.read_retired_archive(content, record, deleted)

    def test_unsafe_member_paths_are_rejected(self):
        for name in ('../retired.c', '/retired.c', 'a/../retired.c', 'a//retired.c', 'a\\retired.c'):
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, 'unsafe retired'):
                checker.read_retired_archive(*self.archive((name,)))

    def test_symlink_directory_and_unexpected_compression_are_rejected(self):
        for names, mode, compression in [(('retired.c',), 0o120777, zipfile.ZIP_DEFLATED),
                                         (('retired/',), 0o40755, zipfile.ZIP_DEFLATED),
                                         (('retired.c',), 0o100644, zipfile.ZIP_STORED)]:
            with self.subTest(mode=mode, compression=compression), self.assertRaisesRegex(ValueError, 'unsafe retired'):
                checker.read_retired_archive(*self.archive(names, mode, compression))

    def test_total_and_individual_size_are_bounded(self):
        content, record, deleted = self.archive()
        with self.assertRaisesRegex(ValueError, 'total uncompressed'):
            checker.read_retired_archive(content, dict(record, uncompressed_bytes=7), deleted)
        deleted['retired.c']['bytes'] = 7
        with self.assertRaisesRegex(ValueError, 'member size changed'):
            checker.read_retired_archive(content, record, deleted)

    def test_member_sha_and_original_blob_are_checked(self):
        for field in ('sha256', 'blob'):
            content, record, deleted = self.archive()
            deleted['retired.c'][field] = '0' * len(deleted['retired.c'][field])
            with self.subTest(field=field), self.assertRaises(ValueError):
                checker.read_retired_archive(content, record, deleted)

    def test_actual_decompressed_length_cannot_exceed_budget(self):
        content, record, deleted = self.archive()
        with mock.patch.object(zipfile.ZipFile, 'open', return_value=io.BytesIO(b'originXX')):
            with self.assertRaisesRegex(ValueError, 'exceeds limit'):
                checker.read_retired_archive(content, record, deleted)


if __name__ == '__main__':
    unittest.main()
