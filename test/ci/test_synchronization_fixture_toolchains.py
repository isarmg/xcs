"""Synchronization fixture commands must use the repository Rust toolchain."""
import importlib.util
from pathlib import Path
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]


class FirstCargoCommand(Exception):
    """Stop before invoking any compiler in this portable wiring test."""


def load_gate(name):
    spec = importlib.util.spec_from_file_location(
        name, ROOT / '.github/scripts' / f'check-{name}.py'
    )
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class SynchronizationFixtureToolchains(unittest.TestCase):
    def test_external_fixture_inherits_toolchain_before_first_cargo(self):
        gate = load_gate('synchronization')
        calls = []

        def first_command(arguments, directory, *args, **kwargs):
            self.assertEqual(arguments[0], 'cargo')
            self.assertEqual(
                (Path(directory) / 'rust-toolchain.toml').read_bytes(),
                (ROOT / 'rust-toolchain.toml').read_bytes(),
            )
            calls.append(arguments)
            raise FirstCargoCommand

        with mock.patch.object(gate, 'run', first_command), mock.patch(
            'sys.argv', ['check-synchronization.py']
        ):
            with self.assertRaises(FirstCargoCommand):
                gate.main()
        self.assertEqual(len(calls), 1)

    def test_linux_parking_resolves_toolchain_from_repository_cwd(self):
        gate = load_gate('linux-parking')
        calls = []

        def first_command(arguments, **kwargs):
            self.assertEqual(arguments[0], 'cargo')
            self.assertEqual(Path(kwargs['cwd']).resolve(), ROOT)
            self.assertTrue((Path(kwargs['cwd']) / 'rust-toolchain.toml').is_file())
            calls.append(arguments)
            raise FirstCargoCommand

        with mock.patch.object(gate.subprocess, 'run', first_command):
            with self.assertRaises(FirstCargoCommand):
                gate.check(ROOT)
        self.assertEqual(len(calls), 1)


if __name__ == '__main__':
    unittest.main()
