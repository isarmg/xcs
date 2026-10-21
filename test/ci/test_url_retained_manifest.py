"""Validate the actual retained harness manifest, including direct macro imports."""
from pathlib import Path
import importlib.util
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('url_encoding_gate', ROOT / '.github/scripts/check-url-encoding.py')
GATE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GATE)

class RetainedManifestTests(unittest.TestCase):
    def setUp(self):
        self.source = (ROOT / 'src/foundation/url-encoding/Cargo.toml').read_text()
        self.manifest = tomllib.loads(GATE.retained_manifest(self.source, ROOT))

    def test_direct_retained_macro_import_has_explicit_test_dependency(self):
        fixture = (ROOT / 'test/foundation/url-encoding/address/tests/unit.rs').read_text()
        self.assertIn('serialization_derive::Deserialize', fixture)
        dependency = self.manifest['dev-dependencies']['serialization-derive']
        self.assertEqual(dependency['path'], str(ROOT / 'src/foundation/serialization-derive'))
        self.assertNotIn('serialization-derive', self.manifest['dependencies'])
        self.assertEqual(tomllib.loads(self.source)['features']['serialization'],
                         ['parser', 'dep:data-serialization', 'data-serialization/derive'])

    def test_facade_and_json_test_dependencies_use_current_local_sources(self):
        deps = self.manifest['dev-dependencies']
        for name in ('data-serialization', 'json-codec'):
            self.assertEqual(deps[name]['path'], str(ROOT / 'src/foundation' / name))
        self.assertIn('derive', deps['data-serialization']['features'])
        self.assertEqual(self.manifest['workspace'], {})

    def test_exact_retained_targets_and_lightweight_build_profiles(self):
        targets = {row['name']: (row['path'], row['harness']) for row in self.manifest['test']}
        self.assertEqual(targets, {
            'url-unit': ('../../../test/foundation/url-encoding/address/tests/unit.rs', True),
            'domain-unit': ('../../../test/foundation/url-encoding/domain/tests/unitbis.rs', True),
            'url-wpt': ('../../../test/foundation/url-encoding/address/tests/wpt.rs', False),
            'behavior': ('../../../test/url-encoding/behavior.rs', True),
        })
        for profile in ('dev', 'test'):
            self.assertEqual(self.manifest['profile'][profile], {'debug': 0, 'incremental': False})

    def test_building_manifest_does_not_mutate_production_manifest(self):
        before = (ROOT / 'src/foundation/url-encoding/Cargo.toml').read_bytes()
        GATE.retained_manifest(self.source, ROOT)
        self.assertEqual(before, (ROOT / 'src/foundation/url-encoding/Cargo.toml').read_bytes())

if __name__ == '__main__':
    unittest.main()
