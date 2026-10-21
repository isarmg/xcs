"""External entropy fixtures pin Rust before any Cargo invocation."""
import importlib.util
import os
from pathlib import Path
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]


def load(relative):
    spec = importlib.util.spec_from_file_location(Path(relative).stem, ROOT / relative)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class EntropyFixtureToolchains(unittest.TestCase):
    def inspect(self, commands):
        def run(arguments, env=None, expected_error=None):
            manifest = Path(arguments[arguments.index('--manifest-path') + 1])
            self.assertTrue(manifest.is_file())
            self.assertFalse(manifest.is_relative_to(ROOT))
            # Assert while the temporary directory is alive and before the
            # first generate-lockfile call, not merely after fixture creation.
            pin = next((parent / 'rust-toolchain.toml' for parent in manifest.parents
                        if (parent / 'rust-toolchain.toml').is_file()), None)
            self.assertIsNotNone(pin)
            self.assertFalse(pin.is_relative_to(ROOT))
            self.assertEqual(pin.read_bytes(), (ROOT / 'rust-toolchain.toml').read_bytes())
            commands.append(arguments)
        return run

    def test_backend_gate_pins_every_command(self):
        gate = load('.github/scripts/check-linux-random-backends.py')
        commands = []
        with patch.object(gate, 'run', side_effect=self.inspect(commands)), \
             patch.object(gate, 'matrix_environment', return_value={}), \
             patch('sys.argv', ['check-linux-random-backends.py']):
            gate.main()
        self.assertEqual(len(commands), 12)
        self.assertEqual(commands[0][1], 'generate-lockfile')

    def test_consumer_gate_pins_every_command(self):
        gate = load('.github/scripts/check-entropy-consumers.py')
        commands = []
        backend = {'matrix_environment': lambda: {}, 'run': self.inspect(commands)}
        with patch.object(gate.runpy, 'run_path', return_value=backend), \
             patch('sys.argv', ['check-entropy-consumers.py']):
            gate.main()
        self.assertEqual(len(commands), 4)
        self.assertEqual(commands[0][1], 'generate-lockfile')

    def test_migration_probe_pins_every_command(self):
        gate = load('test/entropy/check.py')
        commands = []
        # Unit fixtures have their own copy/layout regression. Exercise the
        # separate migration-probe directory through all no_std/std/WASM calls.
        with patch.object(gate, 'unit'), \
             patch.object(gate, 'run', side_effect=self.inspect(commands)), \
             patch.dict(os.environ, {'RUSTFLAGS': '', 'CARGO_ENCODED_RUSTFLAGS': ''}):
            gate.main()
        self.assertEqual(len(commands), 5)
        self.assertEqual(commands[0][0], 'generate-lockfile')


if __name__ == '__main__':
    unittest.main()
