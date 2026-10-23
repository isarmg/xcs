"""Keep asynchronous notification gates strict and capability scoped."""
import importlib.util
import json
import os
import runpy
from pathlib import Path
import subprocess
import tomllib
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
foundation_path = runpy.run_path(str(ROOT / '.github/scripts/foundation_paths.py'))['package_path']
SPEC = importlib.util.spec_from_file_location('async_notification_gate', ROOT / '.github/scripts/check-async-notification.py')
gate = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(gate)


class AsyncNotificationGateTests(unittest.TestCase):
    def test_direct_sqlite_rejects_notification_but_pool_requires_event_only_std(self):
        absent = {'packages': [], 'resolve': {'nodes': []}}
        present = {'packages': [{'name': 'async-notification', 'id': 'event'}],
                   'resolve': {'nodes': [{'id': 'event', 'features': ['std']}]}}
        gate.verify_consumer_features(absent, 'sqlite-access', None)
        gate.verify_consumer_features(present, 'sqlite-access', False)
        with self.assertRaises(AssertionError):
            gate.verify_consumer_features(present, 'sqlite-access', None)
        with self.assertRaises(AssertionError):
            gate.verify_consumer_features(absent, 'sqlite-access', False)
        for features in ([], ['std', 'locks']):
            present['resolve']['nodes'][0]['features'] = features
            with self.assertRaises(AssertionError):
                gate.verify_consumer_features(present, 'sqlite-access', False)

    def test_gate_checks_direct_and_optional_pool_graphs_and_preserves_runtime_contracts(self):
        current = {}
        metadata_consumers = []
        runtime_consumers = []

        def prepare(work, dependencies, source, tests=()):
            current.clear()
            current.update(tomllib.loads('[dependencies]\n' + dependencies)['dependencies'])
            if any(name == 'sqlite_close' for name, _ in tests):
                runtime_consumers.append(current.copy())

        def run(args, directory, **kwargs):
            if args[0] == 'tree':
                features = current['async-notification']['features']
                target = args[args.index('--target') + 1]
                return 'parking v2.2.1' if 'std' in features and target.startswith('x86_64') else ''
            return ''

        def metadata(*args, **kwargs):
            metadata_consumers.append(current.copy())
            if 'sqlite-access' in current and 'pool' not in current['sqlite-access']['features']:
                return json.dumps({'packages': [], 'resolve': {'nodes': []}})
            features = ['std', 'locks'] if 'reactive-state' in current else ['std']
            return json.dumps({'packages': [{'name': 'async-notification', 'id': 'event'}],
                               'resolve': {'nodes': [{'id': 'event', 'features': features}]}})

        with patch.object(gate, 'prepare', side_effect=prepare), \
             patch.object(gate, 'run', side_effect=run) as commands, \
             patch.object(gate.subprocess, 'check_output', side_effect=metadata):
            gate.check(ROOT)
        self.assertEqual([row.get('sqlite-access', {}).get('features') for row in metadata_consumers],
                         [['sqlite-bundled'], ['sqlite-bundled', 'pool'], None])
        self.assertEqual(len(runtime_consumers), 1)
        self.assertEqual(runtime_consumers[0]['sqlite-access']['features'], ['sqlite-bundled', 'pool'])
        self.assertEqual([call.kwargs['minimum_tests'] for call in commands.call_args_list
                          if 'minimum_tests' in call.kwargs], [13, 8])

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
            manifest = tomllib.loads((foundation_path(ROOT, name) / 'Cargo.toml').read_text())
            dep = manifest['dependencies']['async-notification']
            self.assertEqual('locks' in dep.get('features', []), locks)
            self.assertTrue(dep['default-features'])

    def test_std_does_not_enable_locks(self):
        features = tomllib.loads((ROOT / 'src/foundation/concurrency/async-notification/Cargo.toml').read_text())['features']
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
