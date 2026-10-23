"""Keep UUID's advertised feature surface and its real consumer gates honest."""
from contextlib import redirect_stdout, redirect_stderr
import importlib.util
import io
import os
from pathlib import Path
import subprocess
import sys
import tomllib
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
with patch.object(sys, 'path', [str(ROOT / '.github/scripts'), *sys.path]):
    SPEC = importlib.util.spec_from_file_location('uuid_features_gate', ROOT / '.github/scripts/check-uuid-format.py')
    GATE = importlib.util.module_from_spec(SPEC)
    SPEC.loader.exec_module(GATE)


class UuidFeatureContracts(unittest.TestCase):
    def test_only_supported_features_are_advertised(self):
        manifest = tomllib.loads((ROOT / 'src/foundation/core-types/uuid/Cargo.toml').read_text())
        self.assertEqual(manifest['features'], {
            'default': ['std'],
            'std': ['wasm-bindgen?/std', 'js-sys?/std'],
            'js': ['dep:wasm-bindgen', 'dep:js-sys'],
            'rng': ['dep:system-entropy'],
            'serialization': ['dep:data-serialization'],
            'v4': ['rng'], 'v7': ['rng'], 'v8': [],
        })
        self.assertEqual(set(GATE.RETIRED_FEATURES), {
            'atomic', 'borsh', 'macro-diagnostics', 'md5', 'rng-system-entropy',
            'sha1', 'v1', 'v3', 'v5', 'v6',
        })

    def run_gate(self, response=None):
        calls = []

        def run(command, **kwargs):
            calls.append((command, kwargs))
            manifest = Path(command[command.index('--manifest-path') + 1])
            definition = tomllib.loads(manifest.read_text())
            self.assertEqual(definition['dependencies']['uuid'], {
                'path': str(ROOT / 'src/foundation/core-types/uuid'),
                'features': ['v4', 'js'],
            })
            self.assertEqual(definition['test'][0]['path'], str(ROOT / 'test/ci/fixtures/uuid-features.rs'))
            self.assertEqual((manifest.parent / 'rust-toolchain.toml').read_bytes(),
                             (ROOT / 'rust-toolchain.toml').read_bytes())
            self.assertEqual(kwargs['cwd'], ROOT)
            self.assertIn('--offline', command)
            if command[1] != 'generate-lockfile':
                self.assertIn('--locked', command)
            if response is not None:
                result = response(command, kwargs)
                if result is not None:
                    return result
            if '--list' in command:
                return subprocess.CompletedProcess(command, 0,
                    '\n'.join(f'{name}: test' for name in GATE.CONSUMER_TESTS), '')
            if '--features' in command and not kwargs['check']:
                feature = command[command.index('--features') + 1].removeprefix('uuid/')
                self.assertFalse(kwargs['check'])
                return subprocess.CompletedProcess(command, 101, '',
                    f'package `uuid-feature-contracts` depends on `uuid` with feature `{feature}` '
                    'but `uuid` does not have that feature.')
            self.assertTrue(kwargs['check'])
            return subprocess.CompletedProcess(command, 0, '', '')

        with patch.object(GATE.subprocess, 'run', side_effect=run), \
                redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
            GATE.check_features(ROOT)
        return calls

    def test_native_wasm_and_each_retired_feature_are_checked(self):
        with patch.dict(os.environ, {'CARGO_TARGET_DIR': '/chosen/uuid-target'}):
            calls = self.run_gate()
        self.assertEqual(len(calls), 5 + len(GATE.RETIRED_FEATURES))
        self.assertEqual(calls[1][0][-2:], ['--', '--list'])
        self.assertEqual(calls[2][0][1:5], ['test', '--locked', '--test', 'contracts'])
        self.assertEqual(calls[3][0][1:6], ['check', '--locked', '--tests', '--target', 'wasm32-unknown-unknown'])
        self.assertEqual(calls[4][0][1:6], ['check', '--locked', '--tests', '--features',
                                          'uuid/v7,uuid/v8,uuid/serialization'])
        self.assertEqual([command[command.index('--features') + 1] for command, _ in calls[5:]],
                         [f'uuid/{feature}' for feature in GATE.RETIRED_FEATURES])
        for _, kwargs in calls:
            self.assertEqual(kwargs['env']['CARGO_TARGET_DIR'], '/chosen/uuid-target')
        fixture = (ROOT / 'test/ci/fixtures/uuid-features.rs').read_text()
        for name in GATE.CONSUMER_TESTS:
            self.assertIn(f'fn {name}()', fixture)

    def test_missing_test_discovery_fails_closed(self):
        def response(command, _kwargs):
            if '--list' in command:
                return subprocess.CompletedProcess(command, 0, '0 tests', '')
        with self.assertRaisesRegex(AssertionError, 'consumer tests not discovered'):
            self.run_gate(response)

    def test_feature_success_and_unrelated_failures_are_not_rejections(self):
        for code, diagnostic in (
            (0, ''),
            (101, 'network unavailable'),
            (101, '`uuid` with feature `unrelated` but `uuid` does not have that feature'),
            (101, '`other` with feature `atomic` but `other` does not have that feature'),
        ):
            with self.subTest(code=code, diagnostic=diagnostic):
                def response(command, kwargs):
                    if '--features' in command and not kwargs['check']:
                        return subprocess.CompletedProcess(command, code, '', diagnostic)
                with self.assertRaisesRegex(AssertionError, 'accepted or failed for another reason'):
                    self.run_gate(response)

    def test_consumer_compile_failures_are_not_hidden(self):
        def response(command, _kwargs):
            if '--target' in command:
                raise subprocess.CalledProcessError(101, command)
        with self.assertRaises(subprocess.CalledProcessError):
            self.run_gate(response)

    def test_feature_checks_stay_connected_to_foundation_gate(self):
        self.assertIn('check-uuid-format.py', (ROOT / '.github/scripts/check-foundation.sh').read_text())
        with patch.object(GATE, 'check_features', side_effect=RuntimeError('feature gate entered')):
            with self.assertRaisesRegex(RuntimeError, 'feature gate entered'):
                GATE.main()


if __name__ == '__main__':
    unittest.main()
