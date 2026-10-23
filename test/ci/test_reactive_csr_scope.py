"""Keep the maintained reactive runtime on its supported stable CSR boundary."""
import ast
import importlib.util
import subprocess
from unittest import mock
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]


class ReactiveCsrScope(unittest.TestCase):
    def manifest(self, path):
        return tomllib.loads((ROOT / path / 'Cargo.toml').read_text())

    def test_removed_modes_have_no_feature_or_source_branches(self):
        package = ROOT / 'src/foundation/web-runtime/reactive-state'
        manifest = self.manifest('src/foundation/web-runtime/reactive-state')
        for feature in ('hydration', 'subsecond', 'nightly'):
            self.assertNotIn(feature, manifest['features'])
            for path in [*(package / 'src').rglob('*.rs'),
                         *(ROOT / 'test/foundation/reactive-state').rglob('*.rs')]:
                self.assertNotIn(f'feature = "{feature}"', path.read_text(), str(path))
        self.assertNotIn('build', manifest['package'])
        self.assertNotIn('rustc_version', manifest.get('build-dependencies', {}))
        self.assertFalse((package / 'build.rs').exists())
        self.assertFalse((package / 'src/nightly.rs').exists())
        self.assertNotIn('hydration', self.manifest('src/foundation/web-runtime/dom-helpers')['features'])

    def test_dependencies_match_browser_callback_consumers(self):
        self.assertNotIn('async-trait', self.manifest('src/foundation/web-runtime/view-renderer')['dependencies'])
        for package in ('interface', 'web'):
            timers = self.manifest('src/' + package)['dependencies']['gloo-timers']
            self.assertNotIn('futures', timers.get('features', []))

    def test_retained_unit_gate_uses_real_tests_and_test_only_builder(self):
        manifest = self.manifest('src/foundation/web-runtime/reactive-state')
        self.assertNotIn('typed-builder', manifest['dependencies'])
        self.assertEqual(manifest['dev-dependencies']['typed-builder']['path'], '../../code-generation/typed-builder')
        spec = importlib.util.spec_from_file_location('csr_gate', ROOT / '.github/scripts/check-csr-scope.py')
        gate = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(gate)
        calls = []

        def cargo(*args, **kwargs):
            calls.append(args)
            isolated = Path(args[args.index('--manifest-path') + 1])
            parsed = tomllib.loads(isolated.read_text())
            self.assertEqual(parsed['dev-dependencies']['typed-builder']['path'],
                             str(ROOT / 'src/foundation/code-generation/typed-builder'))
            fixture = isolated.parent / '../../../../test/foundation/reactive-state/unit/src/into_reactive_value/tests_21.rs'
            self.assertEqual(fixture.read_bytes(),
                             (ROOT / 'test/foundation/reactive-state/unit/src/into_reactive_value/tests_21.rs').read_bytes())
            self.assertTrue((isolated.parents[4] / 'rust-toolchain.toml').is_file())
            return subprocess.CompletedProcess(args, 0, 'test result: ok. 14 passed; 0 failed\n', '')

        with mock.patch.object(gate, 'cargo', side_effect=cargo):
            gate.check_reactive_units()
        tests = [args for args in calls if args[0] == 'test']
        self.assertEqual(len(tests), 2)
        self.assertTrue(all('--lib' in args and '--locked' in args for args in tests))
        self.assertNotIn('--features', tests[0])
        self.assertEqual(tests[1][-2:], ('--features', 'effects'))

    def test_scope_gate_exercises_retired_modes_and_keeps_template_probe(self):
        source = (ROOT / '.github/scripts/check-csr-scope.py').read_text()
        tree = ast.parse(source)
        assignments = {node.targets[0].id: ast.literal_eval(node.value)
                       for node in ast.walk(tree) if isinstance(node, ast.Assign)
                       and isinstance(node.targets[0], ast.Name)
                       and node.targets[0].id in ('retired', 'retired_features')}
        for feature in ('hydration', 'subsecond', 'nightly'):
            self.assertIn(feature, assignments['retired_features']['reactive-state'])
        self.assertIn('hydration', assignments['retired_features']['dom-helpers'])
        for name in ('owner_shared_context', 'owner_hydration', 'owner_no_hydration', 'hydration_marker'):
            self.assertIn(name, assignments['retired'])
        self.assertIn('test/tools/web-bindings/fixtures/csr-template.rs', source)
        self.assertTrue((ROOT / 'src/foundation/web-runtime/view-renderer/src/hydration.rs').exists())


if __name__ == '__main__':
    unittest.main()
