"""Keep the synchronization gate strict and registered exactly once."""
import importlib.util
import json
from pathlib import Path
import subprocess
import tomllib
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    'synchronization_gate', ROOT / '.github/scripts/check-synchronization.py'
)
gate = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(gate)


class SynchronizationGateTests(unittest.TestCase):
    def test_registered_once_in_platform(self):
        rows = json.loads((ROOT / 'test/ci/foundation-commands.json').read_text())
        entry = {'tool': 'python3', 'args': ['.github/scripts/check-synchronization.py']}
        self.assertEqual(sum(values.count(entry) for values in rows.values()), 1)
        self.assertIn(entry, rows['foundation-platform'])

    def test_full_gate_invokes_crypto_namespace_guard(self):
        # Execute the entry point, rather than merely looking for its spelling.
        with patch('sys.argv', ['check-synchronization.py']), \
                patch.object(gate, 'check_matrix'), \
                patch.object(gate, 'check_behavior'), \
                patch.object(gate, 'check_consumers'), \
                patch.object(gate, 'check_crypto_namespace') as crypto:
            gate.main()
        crypto.assert_called_once()

    def test_crypto_namespace_guard_runs_real_fixture(self):
        import tempfile
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            (directory / 'tests').mkdir()
            def inspect_fixture(*args):
                self.assertEqual(
                    (directory / 'tests/crypto_namespace.rs').read_bytes(),
                    (gate.FIXTURES / 'crypto_namespace.rs').read_bytes(),
                )

            with patch.object(gate, 'prepare') as prepare, \
                    patch.object(gate, 'run', side_effect=inspect_fixture) as run:
                gate.check_crypto_namespace(directory)
            dependencies = prepare.call_args.kwargs['extra_dependencies']
            for name in ('cryptographic-core', 'sha2', 'hmac', 'blake2'):
                self.assertIn(name + '={path=', dependencies)
            self.assertEqual(run.call_args.args[0], [
                'cargo', 'test', '--offline', '--locked', '--jobs', '1',
                '--test', 'crypto_namespace', '--', '--test-threads=1',
            ])
            self.assertFalse((directory / 'tests/crypto_namespace.rs').exists())

    def test_no_std_and_low_level_boundaries(self):
        manifest = tomllib.loads((ROOT / 'src/foundation/concurrency/synchronization/Cargo.toml').read_text())
        self.assertEqual(manifest['features']['default'], ['locks'])
        self.assertNotIn('locks', manifest['features']['parking'])
        self.assertNotIn('std', manifest['features']['arc_lock'])
        self.assertNotIn('deadlock_detection', manifest['features'])

    def test_real_behavior_assertions(self):
        source = (gate.FIXTURES / 'behavior.rs').read_text()
        for name in (
            'contention_and_fair_handoff', 'mapped_and_owned_guards',
            'recursive_and_upgrade_contracts', 'predicate_wakeup_and_timeout',
            'no_poison_after_panic', 'serialization_round_trip',
        ):
            self.assertIn('fn ' + name, source)

    def test_check_uses_uncolored_diagnostics(self):
        result = subprocess.CompletedProcess([], 0, 'ok')
        with patch.object(gate.subprocess, 'run', return_value=result) as execute:
            gate.run(['cargo', 'check', '--offline', '--locked'], ROOT)
        self.assertEqual(
            execute.call_args.args[0],
            ['cargo', '--color', 'never', 'check', '--offline', '--locked'],
        )

    def test_negative_probe_rejects_unexpected_success(self):
        result = subprocess.CompletedProcess([], 0, 'expected text')
        with patch.object(gate.subprocess, 'run', return_value=result):
            with self.assertRaises(RuntimeError):
                gate.run(['cargo', 'check'], ROOT, False, 'expected text')

    def test_negative_probe_requires_exact_reason(self):
        result = subprocess.CompletedProcess([], 1, 'unrelated missing dependency')
        with patch.object(gate.subprocess, 'run', return_value=result):
            with self.assertRaisesRegex(RuntimeError, 'Required diagnostic missing'):
                gate.run(['cargo', 'check'], ROOT, False, 'non-Send guard')

    def test_positive_probe_rejects_warnings(self):
        result = subprocess.CompletedProcess([], 0, 'warning: unused function')
        with patch.object(gate.subprocess, 'run', return_value=result):
            with self.assertRaisesRegex(RuntimeError, 'Unexpected warning'):
                gate.run(['cargo', 'check'], ROOT)

    def test_negative_probe_accepts_matching_failure(self):
        result = subprocess.CompletedProcess([], 1, 'non-Send guard')
        with patch.object(gate.subprocess, 'run', return_value=result):
            self.assertEqual(
                gate.run(['cargo', 'check'], ROOT, False, 'non-Send guard'),
                'non-Send guard',
            )


if __name__ == '__main__':
    unittest.main()
