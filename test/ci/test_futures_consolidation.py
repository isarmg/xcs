"""Futures dependency and coverage boundaries must not silently broaden."""
from pathlib import Path
import runpy
from types import SimpleNamespace
import tomllib
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
foundation_path = runpy.run_path(str(ROOT / '.github/scripts/foundation_paths.py'))['package_path']
GATE = runpy.run_path(str(ROOT / '.github/scripts/check-futures-consolidation.py'))


class FuturesConsolidationTests(unittest.TestCase):
    def test_bare_traits_have_no_dependency(self):
        GATE['validate_dependencies']({'futures', 'futures-probe'}, ())
        for extra in ('slab', 'pin-project-lite', 'futures-macro', 'futures-core'):
            with self.subTest(extra=extra), self.assertRaises(AssertionError):
                GATE['validate_dependencies']({'futures', 'futures-probe', extra}, ())

    def test_shared_state_and_macros_require_explicit_features(self):
        for features in (('alloc',), ('std',), ('channel',), ('utilities',)):
            for extra in ('slab', 'futures-macro'):
                with self.subTest(features=features, extra=extra), self.assertRaises(AssertionError):
                    GATE['validate_dependencies']({'futures', extra}, features)
        GATE['validate_dependencies']({'futures', 'slab', 'pin-project-lite'}, ('shared-state',))
        GATE['validate_dependencies']({'futures', 'futures-macro', 'pin-project-lite'}, ('async-await-macro',))

    def test_native_consumer_graph_rejects_unused_features_and_empty_output(self):
        validate = GATE['validate_native_consumer_features']
        for selected in ('alloc,std,utilities', 'alloc,channel,executor,sink,std,utilities'):
            validate(f'futures v0.3.34 (/local/futures)|{selected}\n', 'consumer')
        for output in ('', 'futures-intrusive v0.5.0|alloc,std\n',
                       'futures v0.3.34|std,utilities\nfutures-macro v0.3.34 (proc-macro)|\n',
                       *(f'futures v0.3.34|std,utilities,{feature}\n'
                         for feature in ('async-await', 'async-await-macro', 'shared-state')),
                       'futures v0.3.34|std,utilities\nfutures v0.3.34|shared-state (*)\n'):
            with self.subTest(output=output), self.assertRaises(AssertionError):
                validate(output, 'consumer')

    def test_native_consumers_are_checked_separately_with_and_without_dev_edges(self):
        check = GATE['check_native_consumer_features']
        run = mock.Mock(return_value=SimpleNamespace(stdout='futures v0.3.34|std,utilities\n'))
        with mock.patch.dict(check.__globals__, run=run):
            check(ROOT)
        selections = []
        for call in run.call_args_list:
            root, command = call.args
            self.assertEqual(root, ROOT)
            self.assertEqual(command.count('-p'), 1)
            self.assertIn('--locked', command)
            self.assertIn('--offline', command)
            self.assertEqual(command[command.index('--target') + 1], 'x86_64-unknown-linux-gnu')
            selections.append((command[command.index('-p') + 1], command[command.index('--edges') + 1]))
        self.assertEqual(selections, [(package, edges)
                                     for package in ('storage-client', 'storage-server')
                                     for edges in ('normal,build', 'normal,build,dev')])

    def test_native_declarations_do_not_change_browser_capabilities(self):
        for package in ('client', 'server'):
            manifest = tomllib.loads((ROOT / f'src/{package}/Cargo.toml').read_text())
            dependency = manifest['dependencies']['futures']
            self.assertFalse(dependency['default-features'])
            self.assertEqual(set(dependency['features']), {'std', 'utilities'})
        browser = tomllib.loads((ROOT / 'src/interface/Cargo.toml').read_text())
        self.assertTrue({'async-await-macro', 'shared-state'} <=
                        set(browser['dependencies']['futures']['features']))

    def test_dormant_tests_are_explicit_and_preserved(self):
        unavailable = GATE['UNAVAILABLE_TESTS']
        self.assertEqual(len(unavailable), 35)
        for path, reasons in unavailable.items():
            with self.subTest(path=path):
                self.assertTrue((ROOT / path).is_file())
                self.assertTrue(reasons)
        self.assertFalse(any((foundation_path(ROOT, name)).exists() for name in GATE['ABSORBED']))

    def test_empty_or_partial_retained_run_is_rejected(self):
        for output in ('', 'test result: ok. 0 passed; 0 failed; 0 ignored\n' * 31,
                       'test result: ok. 335 passed; 0 failed; 0 ignored\n'):
            with self.subTest(output=output), self.assertRaises(AssertionError):
                GATE['validate_retained_output'](output, 29)
        full = ''.join(f'test result: ok. {count} passed; 0 failed; 0 ignored\n'
                       for count in [8, 96, *([1] * 28), 203])
        GATE['validate_retained_output'](full, 29)

    def test_compilation_consumes_an_offline_generated_lock(self):
        source = (ROOT / '.github/scripts/check-futures-consolidation.py').read_text()
        self.assertIn('["cargo", "generate-lockfile", "--offline", "--manifest-path", str(package / "Cargo.toml")]', source)
        self.assertIn('["cargo", "test", "--locked", "--offline", "--manifest-path", str(package / "Cargo.toml")', source)
        self.assertIn('["cargo", "check", "--locked", "--offline", "--manifest-path", str(all_features)', source)

    def test_proc_macro_stays_a_host_boundary(self):
        macro = tomllib.loads((ROOT / 'src/foundation/concurrency/futures-macro/Cargo.toml').read_text())
        library = tomllib.loads((ROOT / 'src/foundation/concurrency/futures/Cargo.toml').read_text())
        self.assertTrue(macro['lib']['proc-macro'])
        self.assertNotIn('futures', macro['dependencies'])
        self.assertTrue(library['dependencies']['futures-macro']['optional'])
        self.assertNotIn('compat', library['features'])
        self.assertNotIn('io-compat', library['features'])
        self.assertEqual(library['features']['utilities'], ['dep:pin-project-lite'])
        self.assertIn('dep:slab', library['features']['shared-state'])


if __name__ == '__main__':
    unittest.main()
