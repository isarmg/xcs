"""Keep the maintained AST regression's optional build paths actually selected."""
from pathlib import Path
import runpy
import re
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
GATE = runpy.run_path(str(ROOT / '.github/scripts/check-rust-syntax-migration.py'))


class RustSyntaxMigrationGateTests(unittest.TestCase):
    def setUp(self):
        self.manifest = tomllib.loads(GATE['manifest']())

    def test_full_syntax_cfg_matches_manifest_and_real_consumer(self):
        root = ROOT / 'src/foundation/attribute-derive'
        manifest = tomllib.loads((root / 'Cargo.toml').read_text())
        cfgs = set(re.findall(r'feature\s*=\s*"([^"]+)"', (root / 'src/syntax_impls.rs').read_text()))
        self.assertTrue(cfgs <= set(manifest['features']))
        self.assertIn('syntax-full', cfgs)
        consumer = runpy.run_path(str(ROOT / '.github/scripts/check-foundation-consumers.py'))
        self.assertEqual(consumer['FEATURES']['attribute-derive'], ['syntax-full'])

    def test_real_sqlite_generator_and_matching_parser_are_selected(self):
        dependencies = self.manifest['dependencies']
        self.assertEqual(set(dependencies['libsqlite3-sys']['features']),
                         {'bundled', 'loadable_extension', 'buildtime_bindgen'})
        self.assertEqual(dependencies['rust-syntax']['features'], ['full'])
        for name in ('rust-syntax', 'rust-source-format', 'cpu-feature-detect', 'base64ct'):
            self.assertTrue(Path(dependencies[name]['path']).is_dir())
        self.assertEqual(dependencies['axum']['features'], ['macros'])
        self.assertEqual(dependencies['strum']['features'], ['derive'])

    def test_functional_package_library_and_link_identities(self):
        roles = {'rust-syntax': 'rust_syntax',
                 'rust-source-format': 'rust_source_format',
                 'cpu-feature-detect': 'cpu_feature_detect'}
        for package, library in roles.items():
            path = ROOT / 'src/foundation' / package / 'Cargo.toml'
            manifest = tomllib.loads(path.read_text())
            self.assertEqual(manifest['package']['name'], package)
            self.assertEqual(manifest['lib']['name'], library)
            self.assertNotIn(library, self.manifest['dependencies'])
        formatter = tomllib.loads((ROOT / 'src/foundation/rust-source-format/Cargo.toml').read_text())
        self.assertEqual(formatter['package']['links'], 'rust_source_format')
        for obsolete in ('syn', 'prettyplease', 'cpufeatures'):
            self.assertNotIn(obsolete, self.manifest['dependencies'])

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
