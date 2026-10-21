"""Keep maintained consumer harnesses on existing packages and capabilities."""
from pathlib import Path
import runpy
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]


class CompressionConsumerTests(unittest.TestCase):
    def harness(self, name):
        return runpy.run_path(str(ROOT / '.github/scripts' / name))

    def assert_local_dependencies_resolve(self, manifest):
        for name, dependency in manifest['dependencies'].items():
            with self.subTest(dependency=name):
                path = Path(dependency['path'])
                self.assertTrue((path / 'Cargo.toml').is_file(), str(path))
                package = tomllib.loads((path / 'Cargo.toml').read_text())
                self.assertEqual(package['package']['name'], dependency.get('package', name))
                for feature in dependency.get('features', []):
                    self.assertIn(feature, package.get('features', {}))

    def test_serialization_family_uses_unified_codec_capability(self):
        gate = self.harness('check-serialization-family.py')
        manifest = tomllib.loads(gate['manifest'](ROOT))
        dependency = manifest['dependencies']['compression']
        self.assertEqual(dependency['path'], str(ROOT / 'src/foundation/compression'))
        self.assertEqual(dependency['features'], ['serialization'])
        self.assert_local_dependencies_resolve(manifest)
        no_std = tomllib.loads(gate['manifest'](ROOT, no_std=True))
        self.assertNotIn('compression', no_std['dependencies'])
        self.assert_local_dependencies_resolve(no_std)

    def test_serialization_consolidation_consumer_paths_and_features_resolve(self):
        gate = self.harness('check-serialization-consolidation.py')
        manifest = tomllib.loads(gate['manifest'](ROOT, gate['PROFILES']['full'], tests=True))
        self.assert_local_dependencies_resolve(manifest)

    def test_release_builder_uses_streams_without_legacy_backend_flags(self):
        path = ROOT / 'tools/web-build/Cargo.toml'
        manifest = tomllib.loads(path.read_text())
        dependency = manifest['dependencies']['compression']
        self.assertEqual((path.parent / dependency['path']).resolve(),
                         (ROOT / 'src/foundation/compression').resolve())
        self.assertEqual(set(dependency['features']), {'streams', 'simd', 'runtime-detection'})
        for old in ('flate2', 'miniz_oxide', 'adler2', 'simd-adler32'):
            self.assertNotIn(old, manifest['dependencies'])


if __name__ == '__main__':
    unittest.main()
