"""Keep asynchronous notification gates strict and capability scoped."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tomllib
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('async_notification_gate', ROOT / '.github/scripts/check-async-notification.py')
gate = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(gate)


class AsyncNotificationGateTests(unittest.TestCase):
    def test_temporary_fixture_inherits_toolchain_before_first_cargo_command(self):
        class FirstCargoCommand(Exception):
            pass

        def first_command(args, directory, **kwargs):
            self.assertEqual(
                (Path(directory) / 'rust-toolchain.toml').read_bytes(),
                (ROOT / 'rust-toolchain.toml').read_bytes(),
            )
            self.assertEqual(args, ['generate-lockfile', '--offline'])
            raise FirstCargoCommand

        with patch.dict(os.environ), patch.object(gate, 'run', side_effect=first_command) as run:
            with self.assertRaises(FirstCargoCommand):
                gate.main()
        run.assert_called_once()

    def test_registered_once(self):
        entry = {'tool': 'python3', 'args': ['.github/scripts/check-async-notification.py']}
        rows = json.loads((ROOT / 'test/ci/foundation-commands.json').read_text())
        self.assertEqual(sum(items.count(entry) for items in rows.values()), 1)
        self.assertIn(entry, rows['foundation-async'])

    def test_sqlite_does_not_request_locks(self):
        for name, locks in (('sqlite-access', False), ('reactive-state', True)):
            manifest = tomllib.loads((ROOT / f'src/foundation/{name}/Cargo.toml').read_text())
            dep = manifest['dependencies']['async-notification']
            self.assertEqual('locks' in dep.get('features', []), locks)
            self.assertTrue(dep['default-features'])

    def test_std_does_not_enable_locks(self):
        features = tomllib.loads((ROOT / 'src/foundation/async-notification/Cargo.toml').read_text())['features']
        self.assertEqual(features, {'default': ['std'], 'std': ['dep:parking'], 'locks': []})

    def test_negative_probe_requires_failure_and_diagnostic(self):
        for result in (subprocess.CompletedProcess([], 0, 'missing'), subprocess.CompletedProcess([], 1, 'other')):
            with patch.object(gate.subprocess, 'run', return_value=result):
                with self.assertRaises(AssertionError):
                    gate.run(['check'], ROOT, expected='missing')

    def test_unexpected_failure_is_fatal(self):
        with patch.object(gate.subprocess, 'run', return_value=subprocess.CompletedProcess([], 1, 'failed')):
            with self.assertRaises(AssertionError):
                gate.run(['test'], ROOT)

    def test_test_count_cannot_silently_drop_coverage(self):
        result = subprocess.CompletedProcess([], 0, 'test result: ok. 0 passed; 0 failed')
        with patch.object(gate.subprocess, 'run', return_value=result):
            with self.assertRaises(AssertionError):
                gate.run(['test'], ROOT, minimum_tests=1)

    def test_warnings_are_not_silently_accepted(self):
        result = subprocess.CompletedProcess([], 0, 'warning: unexpected unused boundary')
        with patch.object(gate.subprocess, 'run', return_value=result):
            with self.assertRaises(AssertionError):
                gate.run(['check'], ROOT)


if __name__ == '__main__':
    unittest.main()
