"""Structural regressions for the unified SQLite package and safe query boundary."""
from pathlib import Path
import json
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[3]

class SqliteAccess(unittest.TestCase):
    def test_single_package_and_separate_native_binding(self):
        base = ROOT / 'src/foundation/data-storage'
        self.assertTrue((base / 'sqlite-access/Cargo.toml').is_file())
        self.assertTrue((base / 'libsqlite3-sys/Cargo.toml').is_file())
        for name in ('sqlx', 'sqlx-core', 'sqlx-sqlite'):
            self.assertFalse((base / name).exists())
        manifest = tomllib.loads((base / 'sqlite-access/Cargo.toml').read_text())
        self.assertEqual(manifest['package']['name'], 'sqlite-access')
        self.assertFalse({'sqlx', 'sqlx-core', 'sqlx-sqlite'} & manifest['dependencies'].keys())

    def test_unsafe_is_confined_to_driver(self):
        base = ROOT / 'src/foundation/data-storage/sqlite-access/src'
        self.assertIn('#![deny(unsafe_code)]', (base / 'lib.rs').read_text())
        self.assertIn('#![forbid(unsafe_code)]', (base / 'query_engine/mod.rs').read_text())
        self.assertIn('#[allow(unsafe_code)]\npub mod sqlite;', (base / 'lib.rs').read_text())
        self.assertIn('mod query_engine;', (base / 'lib.rs').read_text())
        self.assertNotIn('pub mod query_engine;', (base / 'lib.rs').read_text())

    def test_original_test_origins_remain_distinct(self):
        catalog = json.loads((ROOT / 'compliance/rust-sources.json').read_text())
        owner = catalog['external_tests']['src/foundation/data-storage/sqlite-access']
        self.assertEqual(len({item['origin_directory'] for item in owner['files']}), 3)
        for name in ('facade', 'query_engine', 'sqlite'):
            self.assertTrue((ROOT / 'test/foundation/sqlite-access' / name).is_dir())

    def test_regexp_tests_keep_explicit_isolated_databases(self):
        text = (ROOT / 'test/foundation/sqlite-access/sqlite/unit/src/regexp/tests_173.rs').read_text()
        self.assertEqual(text.count('#[test]'), 4)
        self.assertIn('sqlite://:memory:', text)
        self.assertIn('test_regexp_filters_correctly', text)
        self.assertIn('test_regexp_coerces_non_text_values', text)
        self.assertIn('test_invalid_regexp_should_fail', text)

if __name__ == '__main__':
    unittest.main()
