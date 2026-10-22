#!/usr/bin/env python3
"""Exercise fail-closed validation-only archive restoration, including python -O."""
import base64
import gzip
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
SPEC = importlib.util.spec_from_file_location('retired_sources', ROOT / '.github/scripts/retired_sources.py')
ORACLE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ORACLE)


class RetiredSourceOracle(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='retired-oracle-test-')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name) / 'repository'
        self.destination = Path(self.temporary.name) / 'restored'
        (self.root / 'compliance').mkdir(parents=True)
        self.owner = 'src/foundation/ryu'
        self.catalog = json.loads((ROOT / 'compliance/rust-sources.json').read_text())
        self.path = Path('compliance/ryu-1.0.23-retirement.json.gz')
        self.archive = json.loads(gzip.decompress((ROOT / self.path).read_bytes()))

    def restore(self):
        (self.root / 'compliance/rust-sources.json').write_text(json.dumps(self.catalog))
        (self.root / self.path).write_bytes(gzip.compress(json.dumps(self.archive).encode(), mtime=0))
        ORACLE.restore(self.root, self.destination, ['ryu'])

    def rejected(self, pattern):
        with self.assertRaisesRegex(ValueError, pattern):
            self.restore()
        self.assertFalse(self.destination.exists(), 'corrupt archive partially materialized')

    def test_restores_complete_authenticated_material(self):
        self.restore()
        actual = {p.relative_to(self.destination).as_posix() for p in self.destination.rglob('*') if p.is_file()}
        self.assertEqual(actual, set(self.archive['files']))
        for relative, encoded in self.archive['files'].items():
            self.assertEqual((self.destination / relative).read_bytes(), base64.b64decode(encoded))

    def test_rejects_changed_bytes(self):
        first = next(iter(self.archive['files']))
        self.archive['files'][first] = base64.b64encode(b'corrupt').decode()
        self.rejected('digest drift')

    def test_rejects_missing_file(self):
        self.archive['files'].pop(next(iter(self.archive['files'])))
        self.rejected('inventory drift')

    def test_rejects_changed_original_receipt(self):
        self.archive['package']['license'] = 'unverified'
        self.rejected('package receipt drift')

    def test_rejects_changed_retirement_receipt(self):
        self.archive['retirement']['bytes'] += 1
        self.rejected('retirement receipt drift')

    def test_rejects_path_escape_even_with_matching_digest(self):
        relative, data = '../../../escape', b'not allowed'
        row = {'path': relative, 'sha256': hashlib.sha256(data).hexdigest()}
        for external in [self.archive['external_tests'], self.catalog['retired'][self.owner]['retirement']['external_tests']]:
            external['files'].append(dict(row))
            external['bytes'] += len(data)
            external['file_count'] += 1
        self.archive['files'][relative] = base64.b64encode(data).decode()
        self.rejected('path escapes')


if __name__ == '__main__':
    unittest.main()
