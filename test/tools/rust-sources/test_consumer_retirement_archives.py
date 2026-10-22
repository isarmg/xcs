#!/usr/bin/env python3
"""Authenticate complete retired consumer-specialization replay material."""
import base64
import gzip
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[3]


class ConsumerRetirementArchives(unittest.TestCase):
    def test_retired_archives_match_exact_catalog_receipts(self):
        catalog = json.loads((ROOT / 'compliance/rust-sources.json').read_text())
        for name in ('nom', 'minimal-lexical', 'ryu'):
            with self.subTest(package=name):
                directory = 'src/foundation/' + name
                package = dict(catalog['retired'][directory])
                retirement = package.pop('retirement')
                path = ROOT / f"compliance/{name}-{package['version']}-retirement.json.gz"
                archived = json.loads(gzip.decompress(path.read_bytes()))
                self.assertEqual(archived['schema_version'], 1)
                self.assertEqual(archived['package'], package)
                self.assertEqual(archived['external_tests'], retirement['external_tests'])
                self.assertEqual(archived['retirement'], {
                    key: retirement[key] for key in ('bytes', 'category', 'file_count', 'reason')
                })
                expected = {directory + '/' + path: digest
                            for path, digest in package['files_sha256'].items()}
                expected.update({row['path']: row['sha256']
                                 for row in archived['external_tests']['files']})
                self.assertEqual(set(archived['files']), set(expected))
                production_bytes = external_bytes = 0
                for relative, encoded in archived['files'].items():
                    data = base64.b64decode(encoded, validate=True)
                    self.assertEqual(base64.b64encode(data).decode(), encoded)
                    self.assertEqual(hashlib.sha256(data).hexdigest(), expected[relative], relative)
                    self.assertTrue((ROOT / relative).resolve().is_relative_to(ROOT))
                    self.assertFalse((ROOT / relative).exists(), relative)
                    if relative.startswith(directory + '/'):
                        production_bytes += len(data)
                    else:
                        external_bytes += len(data)
                self.assertEqual(production_bytes, retirement['bytes'])
                self.assertEqual(external_bytes, retirement['external_tests']['bytes'])
                self.assertEqual(len(package['files_sha256']), retirement['file_count'])
                self.assertNotIn(directory, catalog['packages'])
                self.assertNotIn(directory, catalog['external_tests'])


if __name__ == '__main__':
    unittest.main()
