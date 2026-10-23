"""Futures dependency and coverage boundaries must not silently broaden."""
from pathlib import Path
import runpy
import tomllib
import unittest

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
