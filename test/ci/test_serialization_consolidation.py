"""Regression guards for the unified serialization implementation and probes."""
import copy
import json
from pathlib import Path
import runpy
import tempfile
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
GATE = runpy.run_path(str(ROOT / '.github/scripts/check-serialization-consolidation.py'))


class SerializationConsolidationTests(unittest.TestCase):
    def test_real_source_has_no_duplicate_or_generated_support(self):
        GATE['validate_source'](ROOT)

    def test_lightweight_profiles_do_not_request_derive_or_path(self):
        for name in ('core', 'alloc', 'rc', 'std'):
            features=GATE['PROFILES'][name]
            self.assertNotIn('derive', features)
            self.assertNotIn('error-path', features)
            manifest=tomllib.loads(GATE['manifest'](ROOT, features))
            self.assertEqual(set(manifest['dependencies']), {'data-serialization'})
            self.assertFalse(manifest['dependencies']['data-serialization']['default-features'])

    def test_generated_consumers_resolve_current_local_packages(self):
        manifest = tomllib.loads(GATE['manifest'](ROOT, GATE['PROFILES']['full'], tests=True))
        dependencies = manifest['dependencies']
        self.assertNotIn('url', dependencies)
        url = dependencies['url-encoding']
        self.assertEqual(Path(url['path']), ROOT / 'src/foundation/networking/url-encoding')
        self.assertFalse(url['default-features'])
        self.assertEqual(url['features'], ['parser', 'std', 'serialization'])
        for name, dependency in dependencies.items():
            self.assertTrue((Path(dependency['path']) / 'Cargo.toml').is_file(), name)

    def test_url_probe_uses_current_public_namespace(self):
        source = (ROOT / 'test/ci/fixtures/serialization-consolidation.rs').read_text()
        self.assertIn('url_encoding::address::Url', source)
        self.assertNotIn('url::Url', source)

    def test_native_and_browser_wasm_profiles_are_real(self):
        self.assertEqual(GATE['TARGETS'], ('x86_64-unknown-linux-gnu', 'wasm32-unknown-unknown'))
        self.assertEqual(len(GATE['PROFILES']), 7)
        self.assertIn('traits::<Result<u128, i128>>()', GATE['source']('core'))
        self.assertIn('core::num::Saturating<u64>', GATE['source']('core'))
        self.assertIn('error_path::Track', GATE['source']('path'))

    def test_build_script_and_minimum_version_fail_closed(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder); package=root/'src/foundation/data-storage/data-serialization'
            (package/'src').mkdir(parents=True)
            manifest=package/'Cargo.toml'
            manifest.write_text('[package]\nbuild=false\nrust-version="1.99"\n')
            (package/'src/lib.rs').write_text('pub struct Support;')
            GATE['validate_source'](root)
            (package/'build.rs').write_text('fn main() {}')
            with self.assertRaisesRegex(AssertionError,'obsolete build script'):
                GATE['validate_source'](root)
            (package/'build.rs').unlink()
            manifest.write_text('[package]\nbuild=false\nrust-version="1.56"\n')
            with self.assertRaisesRegex(AssertionError,'toolchain minimum'):
                GATE['validate_source'](root)

    def test_ci_registration_is_exactly_once(self):
        commands=json.loads((ROOT/'test/ci/foundation-commands.json').read_text())
        wanted={'tool':'python3','args':['.github/scripts/check-serialization-consolidation.py']}
        self.assertEqual(commands['foundation-language'].count(wanted),1)
        self.assertEqual(sum(rows.count(wanted) for rows in commands.values()),1)


if __name__ == '__main__':
    unittest.main()
