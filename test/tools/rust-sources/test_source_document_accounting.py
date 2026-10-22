#!/usr/bin/env python3
"""The source index counts physical files, not repeated origin aliases."""
import json
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools/rust-sources'))
import manage


class SourceDocumentAccounting(unittest.TestCase):
    def test_current_file_count_deduplicates_all_origin_aliases(self):
        catalog = json.loads((ROOT / manage.CATALOG).read_text())
        policy = json.loads((ROOT / manage.POLICY).read_text())
        owners = [*catalog['packages'].values(), *catalog.get('merges', [])]
        paths = {owner['directory'] + '/' + path
                 for owner in owners for path in owner['files_sha256']}
        self.assertLess(len(paths), sum(len(owner['files_sha256']) for owner in owners))
        external = sum(owner['file_count'] for owner in catalog['external_tests'].values())
        document = manage.document(catalog, policy)
        self.assertIn(f'包内当前文件 {len(paths)} 个', document)
        self.assertIn(f'当前来源文件总计 {len(paths) + external} 个', document)

    def test_external_owner_directories_are_not_claimed_as_original_identities(self):
        catalog = json.loads((ROOT / manage.CATALOG).read_text())
        policy = json.loads((ROOT / manage.POLICY).read_text())
        document = manage.document(catalog, policy)
        self.assertIn(f"另有 {len(catalog['external_tests'])} 个归属目录登记的外置测试文件", document)
        self.assertNotIn('个原始身份所属的外置测试文件', document)


if __name__ == '__main__':
    unittest.main()
