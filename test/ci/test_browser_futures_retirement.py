"""The browser executor is real js-sys code, not a renamed compatibility facade."""
import base64
import gzip
import hashlib
import json
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]


class BrowserFuturesRetirement(unittest.TestCase):
    def test_exact_retirement_material_is_preserved(self):
        plan = json.loads((ROOT / 'compliance/browser-futures-retirement-plan.json').read_text())
        self.assertEqual(plan['schema'], 1)
        self.assertEqual(len(plan['packages']), 1)
        item = plan['packages'][0]
        owner = 'src/foundation/wasm-bindgen-futures'
        self.assertEqual(item['directory'], owner)
        self.assertFalse((ROOT / owner).exists())
        archive = json.loads(gzip.decompress(
            (ROOT / 'compliance/wasm-bindgen-futures-0.4.79-retirement.json.gz').read_bytes()))
        self.assertEqual(archive['schema_version'], 1)
        self.assertEqual(archive['package'], item['package'])
        self.assertIsNone(archive['external_tests'])
        self.assertEqual(archive['retirement'], {
            key: item[key] for key in ('bytes', 'category', 'file_count', 'reason')})
        expected = {owner + '/' + path: digest
                    for path, digest in item['package']['files_sha256'].items()}
        self.assertEqual(set(archive['files']), set(expected))
        total = 0
        for path, encoded in archive['files'].items():
            data = base64.b64decode(encoded, validate=True)
            self.assertEqual(hashlib.sha256(data).hexdigest(), expected[path])
            self.assertEqual(base64.b64encode(data).decode(), encoded)
            total += len(data)
        self.assertEqual(total, item['bytes'])
        self.assertEqual(len(expected), item['file_count'])
        for path in item['package']['license_materials']:
            self.assertTrue((ROOT / path).is_file())

    def test_active_manifests_do_not_depend_on_shim(self):
        for path in (ROOT / 'src').rglob('Cargo.toml'):
            self.assertNotIn('wasm-bindgen-futures', path.read_text(), str(path))
        script = (ROOT / '.github/scripts/check-csr-scope.py').read_text()
        self.assertNotIn('wasm-bindgen-futures', script)
        macros = ROOT / 'src/foundation/wasm-binding-macros/src'
        for name in ('ast.rs', 'codegen.rs', 'encode.rs', 'lib.rs'):
            self.assertNotIn('wasm_bindgen_futures', (macros / name).read_text())
        for path in macros.glob('*.rs'):
            self.assertNotIn('WASM_BINDGEN_USE_JS_SYS', path.read_text())
            self.assertNotIn('use_js_sys_futures', path.read_text())

    def test_ci_requires_browser_execution_without_disabling_security(self):
        workflow = (ROOT / '.github/workflows/ci.yml').read_text()
        self.assertIn("if: matrix.suite == 'foundation-language'", workflow)
        self.assertIn('echo "ZXS_CHROMIUM=$chrome" >> "$GITHUB_ENV"', workflow)
        script = (ROOT / '.github/scripts/check-web-bindgen-scope.py').read_text()
        self.assertIn("os.environ.get('ZXS_CHROMIUM')", script)
        for flag in ('--no-sandbox', '--disable-setuid-sandbox', '--disable-web-security'):
            self.assertNotIn(flag, script)

    def test_dispatch_preserves_std_without_enabling_eval(self):
        manifest = tomllib.loads((ROOT / 'src/foundation/task-dispatch/Cargo.toml').read_text())
        dependency = manifest['dependencies']['js-sys']
        self.assertEqual(dependency['path'], '../js-sys')
        self.assertTrue(dependency['optional'])
        self.assertFalse(dependency['default-features'])
        self.assertEqual(dependency['features'], ['std'])
        self.assertEqual(manifest['features']['wasm-bindgen'], ['dep:js-sys'])
        js = tomllib.loads((ROOT / 'src/foundation/js-sys/Cargo.toml').read_text())
        self.assertIn('wasm-bindgen/std', js['features']['std'])


if __name__ == '__main__':
    unittest.main()
