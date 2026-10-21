"""Retired proof replay is exact and cannot hide missing active material."""
import base64
from copy import deepcopy
import gzip
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('retired_platform_history', Path(__file__).resolve().parents[3] / 'tools/rust-sources/historical_test_sources.py')
history = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(history)


def sha(data):
    return hashlib.sha256(data).hexdigest()


class RetiredPlatformArchive(unittest.TestCase):
    def setUp(self):
        routing = patch.object(history.proof_routing, "RECEIPTS", ())
        routing.start()
        self.addCleanup(routing.stop)
        temp = tempfile.TemporaryDirectory(); self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        (self.root / 'compliance').mkdir()
        self.directory = 'src/foundation/entropy-old'
        self.test_root = 'test/foundation/entropy-old'
        self.production = self.directory + '/lib.rs'
        self.test = self.test_root + '/tests.rs'
        self.data = {self.production: b'pub fn entropy() {}\n', self.test: b'assert!(true);\n'}
        self.package = dict(directory=self.directory, origin_directory='vendor/entropy-old',
                            upstream=dict(name='entropy', version='1.0.0'),
                            files_sha256={'lib.rs': sha(self.data[self.production])})
        self.owner = dict(directory=self.directory, origin_directory='vendor/entropy-old',
                          roots=[self.test_root], files=[dict(path=self.test, source=self.directory + '/tests.rs',
                          sha256=sha(self.data[self.test]), source_sha256_before=sha(self.data[self.test]))],
                          file_count=1, bytes=len(self.data[self.test]))
        retired = dict(self.package, retirement=dict(reason='consolidation', category='duplicate', targets=['target'],
                       file_count=1, bytes=len(self.data[self.production]), external_tests=self.owner))
        self.catalog = dict(packages={}, external_tests={}, retired={self.directory: retired}, retired_records_sha256={})
        self.archive = dict(schema_version=1, package=deepcopy(self.package), external_tests=deepcopy(self.owner),
                            retirement=deepcopy(retired['retirement']),
                            files={p: base64.b64encode(b).decode() for p, b in self.data.items()})
        self.save_catalog()
        self.save_archive()
        old = dict(schema_version=1, roots={self.test_root: {self.test: sha(self.data[self.test])}},
                   files=[dict(source=self.directory + '/tests.rs', path=self.test,
                               sha256=sha(self.data[self.test]), before_sha256=sha(self.data[self.test]))])
        self.pin_json('RECEIPT', 'RECEIPT_SHA256', old)
        self.pin_json('CORRECTIONS', 'CORRECTIONS_SHA256', dict(schema_version=1, files=[]))

    def pin_json(self, path_constant, hash_constant, record):
        path = self.root / getattr(history, path_constant)
        path.write_text(json.dumps(record))
        stub = patch.object(history, hash_constant, sha(path.read_bytes()))
        stub.start(); self.addCleanup(stub.stop)

    def save_catalog(self, repin=True):
        if repin:
            self.catalog['retired_records_sha256'] = {p: sha(json.dumps(v, sort_keys=True, ensure_ascii=False).encode())
                for p, v in self.catalog['retired'].items()}
        (self.root / 'compliance/rust-sources.json').write_text(json.dumps(self.catalog))

    def save_archive(self, content=None):
        path = 'compliance/retired.json.gz'
        content = content if content is not None else gzip.compress(json.dumps(self.archive).encode(), mtime=0)
        (self.root / path).write_bytes(content)
        stub = patch.object(history, 'RETIREMENT_ARCHIVES', {self.directory: (path, sha(content))})
        stub.start(); self.addCleanup(stub.stop)

    def test_exact_archive_replays_both_production_and_old_test_view(self):
        read = history.historical_reader(self.root)
        self.assertEqual(read(self.production), self.data[self.production])
        self.assertEqual(read(self.directory + '/tests.rs'), self.data[self.test])
        self.assertEqual(read(self.test), self.data[self.test])
        with self.assertRaises(FileNotFoundError):
            read('src/foundation/unrelated/lib.rs')

    def test_archive_pin_tamper_and_truncation_rejected(self):
        path = self.root / 'compliance/retired.json.gz'; original = path.read_bytes()
        for data in (original + b' ', original[:-5]):
            path.write_bytes(data)
            with self.assertRaisesRegex(ValueError, 'archive changed'): history.retired_view(self.root)

    def test_archived_missing_added_and_altered_bytes_rejected(self):
        original = deepcopy(self.archive)
        for mutation in ('missing', 'added', 'changed'):
            self.archive = deepcopy(original)
            if mutation == 'missing': del self.archive['files'][self.test]
            elif mutation == 'added': self.archive['files'][self.test_root + '/extra'] = 'eA=='
            else: self.archive['files'][self.production] = 'eA=='
            self.save_archive()
            with self.assertRaisesRegex(ValueError, 'inventory mismatch|content mismatch'): history.retired_view(self.root)

    def test_wrong_owner_origin_or_previous_source_receipt_rejected(self):
        original = deepcopy(self.catalog)
        for field, value in (('directory', 'src/foundation/other'), ('origin_directory', 'other'), ('files_sha256', {})):
            self.catalog = deepcopy(original)
            self.catalog['retired'][self.directory][field] = value
            self.save_catalog()
            with self.assertRaisesRegex(ValueError, 'identity/last receipt'): history.retired_view(self.root)

    def test_wrong_external_receipt_and_retired_record_hash_rejected(self):
        self.catalog['retired'][self.directory]['retirement']['external_tests'] = {}
        self.save_catalog(repin=False)
        with self.assertRaisesRegex(ValueError, 'catalog receipt'): history.retired_view(self.root)
        self.save_catalog()
        with self.assertRaisesRegex(ValueError, 'retirement receipt'): history.retired_view(self.root)

    def test_active_owner_cannot_use_retirement_fallback(self):
        self.catalog['packages'][self.directory] = self.package; self.save_catalog()
        with self.assertRaisesRegex(ValueError, 'identity/last receipt'): history.retired_view(self.root)

    def test_production_or_external_root_reappearance_rejected(self):
        for relative in (self.directory, self.test_root):
            path = self.root / relative; path.mkdir(parents=True)
            with self.assertRaisesRegex(ValueError, 'returned|absent root'): history.retired_view(self.root)
            path.rmdir()

    def test_symlinked_root_rejected(self):
        path = self.root / self.directory; path.parent.mkdir(parents=True)
        path.symlink_to(self.root / 'compliance', target_is_directory=True)
        with self.assertRaisesRegex(ValueError, 'symlink'): history.retired_view(self.root)

    def test_counts_and_bytes_must_match_archived_material(self):
        self.catalog['retired'][self.directory]['retirement']['bytes'] += 1; self.save_catalog()
        with self.assertRaisesRegex(ValueError, 'retirement receipt'): history.retired_view(self.root)
        self.archive['retirement']['bytes'] += 1; self.save_archive()
        with self.assertRaisesRegex(ValueError, 'count/byte'): history.retired_view(self.root)

    def test_decompression_and_compressed_size_bounds(self):
        with patch.object(history, 'MAX_RETIREMENT_COMPRESSED', 1):
            with self.assertRaisesRegex(ValueError, 'compressed bound'): history.retired_view(self.root)
        with patch.object(history, 'MAX_RETIREMENT_EXPANDED', 1):
            with self.assertRaisesRegex(ValueError, 'expanded bound'): history.retired_view(self.root)

    def test_unsafe_archived_path_and_invalid_encoding_rejected(self):
        self.archive['files'][self.production] = 'invalid!'; self.save_archive()
        with self.assertRaises(ValueError): history.retired_view(self.root)
        self.archive['files'][self.production] = base64.b64encode(self.data[self.production]).decode()
        self.save_archive()
        with self.assertRaisesRegex(ValueError, 'unsafe'):
            history.historical_reader(self.root)('../escape')

    def test_archive_schema_and_escaping_inventory_rejected(self):
        original = deepcopy(self.archive)
        self.archive['unexpected'] = True; self.save_archive()
        with self.assertRaisesRegex(ValueError, 'schema'): history.retired_view(self.root)
        self.archive = original
        value = self.archive['files'].pop(self.production)
        self.archive['files'][self.directory + '/../escape'] = value
        self.archive['package']['files_sha256'] = {'../escape': sha(self.data[self.production])}
        self.catalog['retired'][self.directory]['files_sha256'] = self.archive['package']['files_sha256']
        self.save_catalog(); self.save_archive()
        with self.assertRaisesRegex(ValueError, 'unsafe'): history.retired_view(self.root)

    def test_old_platform_receipt_still_checks_archived_hash(self):
        record = json.loads((self.root / history.RECEIPT).read_bytes())
        record['roots'][self.test_root][self.test] = '0' * 64
        self.pin_json('RECEIPT', 'RECEIPT_SHA256', record)
        with self.assertRaisesRegex(ValueError, 'inventory/content drift'):
            history.historical_reader(self.root)


if __name__ == '__main__':
    unittest.main()
