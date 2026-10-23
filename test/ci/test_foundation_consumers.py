"""Keep consumer coverage tied to the production property parser after retirement."""
from pathlib import Path
import runpy
import subprocess
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
GATE = runpy.run_path(str(ROOT / '.github/scripts/check-foundation-consumers.py'))


class FoundationConsumerGateTests(unittest.TestCase):
    def test_property_diagnostics_run_real_locked_macro_tests(self):
        names = GATE['PROP_PARSER_TESTS']
        listing = subprocess.CompletedProcess([], 0, '\n'.join(f'{name}: test' for name in names))
        with mock.patch.object(GATE['subprocess'], 'run', return_value=listing) as run:
            GATE['check_prop_parser'](ROOT, Path('/tmp/consumer-target'))
        command = ['cargo', 'test', '--locked', '--offline', '-p', 'view-macros',
                   '--lib', 'prop_options::tests', '--target-dir', '/tmp/consumer-target']
        self.assertEqual(run.call_args_list, [
            mock.call(command + ['--', '--list'], cwd=ROOT, check=True,
                      capture_output=True, text=True),
            mock.call(command + ['--', '--test-threads=1'], cwd=ROOT, check=True),
        ])
        tests = (ROOT / 'test/view-macros/unit/prop_options.rs').read_text()
        for name in names:
            self.assertIn(f'fn {name.rsplit("::", 1)[-1]}()', tests)

    def test_empty_or_partial_discovery_cannot_pass(self):
        for output in ['', next(iter(GATE['PROP_PARSER_TESTS'])) + ': test\n']:
            with self.subTest(output=output):
                listing = subprocess.CompletedProcess([], 0, output)
                with mock.patch.object(GATE['subprocess'], 'run', return_value=listing) as run:
                    with self.assertRaisesRegex(AssertionError, 'not discovered'):
                        GATE['check_prop_parser'](ROOT, Path('/tmp/consumer-target'))
                    self.assertEqual(run.call_count, 1)

    def test_discovery_build_failure_is_not_hidden(self):
        with mock.patch.object(GATE['subprocess'], 'run',
                               side_effect=subprocess.CalledProcessError(101, ['cargo'])):
            with self.assertRaises(subprocess.CalledProcessError):
                GATE['check_prop_parser'](ROOT, Path('/tmp/consumer-target'))


if __name__ == '__main__':
    unittest.main()
