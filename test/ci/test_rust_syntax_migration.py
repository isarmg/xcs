"""Keep the maintained AST regression's optional build paths actually selected."""
from pathlib import Path
import runpy
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
GATE = runpy.run_path(str(ROOT / '.github/scripts/check-rust-syntax-migration.py'))


class RustSyntaxMigrationGateTests(unittest.TestCase):
    def setUp(self):
        self.manifest = tomllib.loads(GATE['manifest']())

    def test_real_sqlite_generator_and_matching_parser_are_selected(self):
        dependencies = self.manifest['dependencies']
        self.assertEqual(set(dependencies['libsqlite3-sys']['features']),
                         {'bundled', 'loadable_extension', 'buildtime_bindgen'})
        self.assertEqual(dependencies['syn']['features'], ['full'])
        for name in ('syn', 'prettyplease', 'cpufeatures', 'base64ct'):
            self.assertTrue(Path(dependencies[name]['path']).is_dir())
        self.assertEqual(dependencies['axum']['features'], ['macros'])
        self.assertEqual(dependencies['strum']['features'], ['derive'])

    def test_preserved_argon2_tests_cannot_silently_compile_to_zero(self):
        features = self.manifest['features']
        self.assertEqual(set(features['default']), {'alloc', 'password-hash'})
        self.assertIn('alloc', features)
        self.assertIn('password-hash', features)
        self.assertIn('password-hash', self.manifest['dependencies'])
        self.assertIn('base64ct', self.manifest['dependencies'])

    def test_five_explicit_test_targets_and_bounded_profiles(self):
        tests = self.manifest['test']
        self.assertEqual({row['name'] for row in tests},
                         {'migration', 'typed-builder-main', 'typed-builder-no-std',
                          'typed-builder-no-leakage', 'argon2-phc'})
        for row in tests:
            self.assertTrue(Path(row['path']).is_file())
        self.assertFalse(self.manifest['lib']['test'])
        self.assertFalse(self.manifest['lib']['doctest'])
        for profile in ('dev', 'test'):
            self.assertEqual(self.manifest['profile'][profile]['debug'], 0)
            self.assertFalse(self.manifest['profile'][profile]['incremental'])


if __name__ == '__main__':
    unittest.main()
