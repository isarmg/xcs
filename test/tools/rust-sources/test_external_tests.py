"""Current external test ownership must survive refresh without silent shrinkage."""
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import subprocess
import sys
import unittest

HELPERS = Path(__file__).resolve().parents[3] / 'tools/rust-sources'
sys.path.insert(0, str(HELPERS))
SPEC = importlib.util.spec_from_file_location('external_tests', HELPERS / 'external_tests.py')
external = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(external)


class ExternalTestAccounting(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.file = self.root / 'test/foundation/mini/tests.rs'
        self.file.parent.mkdir(parents=True)
        self.file.write_text('assertion')
        self.owner = 'src/foundation/mini'
        self.packages = {self.owner: {'origin_directory': 'vendor/mini', 'files_sha256': {}}}
        self.record = {'schema_version': 1, 'owners': [{'directory': self.owner, 'roots': ['test/foundation/mini'], 'files': [{'source': self.owner + '/tests.rs', 'path': 'test/foundation/mini/tests.rs', 'sha256': hashlib.sha256(self.file.read_bytes()).hexdigest(), 'source_sha256_before': 'a' * 64}]}]}
        self.mapping = self.root / 'compliance/external-tests/mini.json'
        self.mapping.parent.mkdir(parents=True)
        self.save()

    def save(self):
        self.mapping.write_text(json.dumps(self.record))

    def test_owner_keeps_original_identity(self):
        owners, receipts = external.load(self.root, self.packages)
        self.assertEqual(owners[self.owner]['origin_directory'], 'vendor/mini')
        self.assertEqual(set(receipts), {'compliance/external-tests/mini.json'})

    def test_missing_changed_added_and_unattributed_files_fail(self):
        for mutation in ('missing', 'changed', 'added', 'unattributed'):
            with self.subTest(mutation=mutation):
                original = self.file.read_bytes()
                extra = None
                if mutation == 'missing': self.file.unlink()
                elif mutation == 'changed': self.file.write_text('different assertion')
                else:
                    extra = self.file.parent / 'extra.rs' if mutation == 'added' else self.file.parent.parent / 'rogue.rs'
                    extra.write_text('unaccounted')
                with self.assertRaises(ValueError): external.load(self.root, self.packages)
                self.file.write_bytes(original)
                if extra: extra.unlink()

    def test_symlink_file_and_parent_are_rejected(self):
        self.file.unlink()
        self.file.symlink_to(self.mapping)
        with self.assertRaisesRegex(ValueError, 'symlink'): external.load(self.root, self.packages)
        self.file.unlink()
        self.file.parent.rmdir()
        self.file.parent.symlink_to(self.mapping.parent, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, 'symlink'): external.load(self.root, self.packages)

    def test_escape_and_wrong_owner_rejected(self):
        row = self.record['owners'][0]['files'][0]
        for field, value in [('path', '../escape'), ('path', '/absolute'), ('source', 'src/foundation/other/tests.rs')]:
            with self.subTest(field=field, value=value):
                old = row[field]; row[field] = value; self.save()
                with self.assertRaises(ValueError): external.load(self.root, self.packages)
                row[field] = old

    def test_refresh_preserves_external_fragments_when_relocating(self):
        owners, _ = external.load(self.root, self.packages)
        with self.assertRaisesRegex(ValueError, 'accounting shrank'):
            external.check_previous(owners, {})
        moved = json.loads(json.dumps(owners))
        moved[self.owner]['files'][0]['path'] = 'test/new/tests.rs'
        external.check_previous(owners, moved)
        owners[self.owner]['files'].append(dict(owners[self.owner]['files'][0]))
        with self.assertRaisesRegex(ValueError, 'accounting shrank'):
            external.check_previous(owners, moved)

    def test_refresh_cannot_silently_remove_source(self):
        previous = {self.owner: {'origin_directory': 'vendor/mini', 'files_sha256': {'tests.rs': 'a' * 64}}}
        with self.assertRaisesRegex(ValueError, 'unaccounted source removal'):
            external.check_removed(previous, self.packages, {})
        owners, _ = external.load(self.root, self.packages)
        external.check_removed(previous, self.packages, owners)


class FullFileRestoration(unittest.TestCase):
    setUp = ExternalTestAccounting.setUp
    save = ExternalTestAccounting.save
    def restoration(self):
        row = self.record['owners'][0]['files'][0]
        row['source_sha256_before'] = row['sha256']
        self.save()
        owners, _ = external.load(self.root, self.packages)
        prior = json.loads(json.dumps(self.packages))
        data = self.file.read_bytes(); self.file.unlink()
        restored = self.root / row['source']
        restored.parent.mkdir(parents=True, exist_ok=True); restored.write_bytes(data)
        self.packages[self.owner]['files_sha256']['tests.rs'] = row['sha256']
        return owners, prior, restored

    def test_exact_full_file_restoration_is_not_source_loss(self):
        owners, prior, _ = self.restoration()
        external.check_previous(owners, {}, self.root, self.packages, prior)

    def test_altered_restoration_rejected(self):
        owners, prior, restored = self.restoration()
        restored.write_bytes(b'changed')
        self.packages[self.owner]['files_sha256']['tests.rs'] = hashlib.sha256(restored.read_bytes()).hexdigest()
        with self.assertRaisesRegex(ValueError, 'accounting shrank'):
            external.check_previous(owners, {}, self.root, self.packages, prior)

    def test_wrong_owner_and_wrong_path_restorations_rejected(self):
        owners, prior, restored = self.restoration()
        self.packages[self.owner]['origin_directory'] = 'different-release'
        with self.assertRaisesRegex(ValueError, 'accounting shrank'):
            external.check_previous(owners, {}, self.root, self.packages, prior)
        self.packages[self.owner]['origin_directory'] = 'vendor/mini'
        restored.rename(restored.with_name('wrong.rs'))
        with self.assertRaisesRegex(ValueError, 'accounting shrank'):
            external.check_previous(owners, {}, self.root, self.packages, prior)

    def test_external_copy_must_disappear(self):
        owners, prior, restored = self.restoration()
        self.file.write_bytes(restored.read_bytes())
        with self.assertRaisesRegex(ValueError, 'accounting shrank'):
            external.check_previous(owners, {}, self.root, self.packages, prior)

    def test_fragment_or_existing_production_copy_cannot_be_dropped(self):
        owners, prior, _ = self.restoration()
        row = owners[self.owner]['files'][0]
        original = row['source_sha256_before']; row['source_sha256_before'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'accounting shrank'):
            external.check_previous(owners, {}, self.root, self.packages, prior)
        row['source_sha256_before'] = original
        prior[self.owner]['files_sha256']['tests.rs'] = row['sha256']
        with self.assertRaisesRegex(ValueError, 'accounting shrank'):
            external.check_previous(owners, {}, self.root, self.packages, prior)

    def test_restored_source_symlink_rejected(self):
        owners, prior, restored = self.restoration()
        data = restored.read_bytes(); restored.unlink()
        target = restored.with_name('target.rs'); target.write_bytes(data); restored.symlink_to(target)
        with self.assertRaisesRegex(ValueError, 'symlink'):
            external.check_previous(owners, {}, self.root, self.packages, prior)


class ExternalSourceAttributes(unittest.TestCase):
    def test_relocated_upstream_bytes_disable_git_text_normalization(self):
        root = Path(__file__).resolve().parents[3]
        paths = ['test/foundation/nu-ansi-term/tests/style.rs',
                 'test/foundation/ring/tests/hmac_tests.rs',
                 'test/tools/web-bindings/fixtures/contract.rs']
        result = subprocess.check_output(['git', 'check-attr', '-z', 'text', '--', *paths], cwd=root)
        fields = result.decode().strip('\0').split('\0')
        self.assertEqual([tuple(fields[i:i + 3]) for i in range(0, len(fields), 3)],
                         [(path, 'text', 'unset') for path in paths])

    def test_original_crlf_sources_keep_text_and_space_checks(self):
        root = Path(__file__).resolve().parents[3]
        paths = ['src/foundation/allocator-api2/src/stable/boxed.rs',
                 'src/foundation/allocator-api2/src/stable/vec/mod.rs']
        result = subprocess.check_output(['git', 'check-attr', '-z', 'text', 'whitespace', '--', *paths], cwd=root)
        fields = result.decode().strip('\0').split('\0')
        rows = [tuple(fields[i:i + 3]) for i in range(0, len(fields), 3)]
        for path in paths:
            self.assertIn((path, 'text', 'unset'), rows)
            self.assertIn((path, 'whitespace', 'blank-at-eol,blank-at-eof,space-before-tab,cr-at-eol'), rows)

    def test_relocated_ring_vectors_preserve_whitespace_exceptions(self):
        root = Path(__file__).resolve().parents[3]
        paths = ['test/foundation/ring/tests/hmac_tests.txt',
                 'test/foundation/ring/cavp/third_party/NIST/SHAVS/SHA256ShortMsg.rsp']
        result = subprocess.check_output(['git', 'check-attr', '-z', 'whitespace', '--', *paths], cwd=root)
        fields = result.decode().strip('\0').split('\0')
        self.assertEqual([fields[i + 2] for i in range(0, len(fields), 3)],
                         ['-blank-at-eof', '-blank-at-eol,-blank-at-eof'])


if __name__ == '__main__':
    unittest.main()
