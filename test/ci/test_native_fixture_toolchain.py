"""Native C fixture builds must retain the repository Rust toolchain."""
import importlib.util
from pathlib import Path
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    'native_c_gate', ROOT / '.github/scripts/check-native-c-toolchain.py')
gate = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(gate)


class NativeFixtureToolchainTests(unittest.TestCase):
    def test_pin_exists_before_first_cargo_command(self):
        class FirstCargoCommand(Exception):
            pass

        def first_command(command, *, cwd, **kwargs):
            self.assertEqual(command[:2], ['cargo', 'build'])
            self.assertEqual(
                (cwd / 'rust-toolchain.toml').read_bytes(),
                (ROOT / 'rust-toolchain.toml').read_bytes(),
            )
            raise FirstCargoCommand

        with patch.object(gate, 'check_preservation'), patch.object(
                gate.subprocess, 'Popen', side_effect=first_command) as cargo:
            with self.assertRaises(FirstCargoCommand):
                gate.check()
        cargo.assert_called_once()


if __name__ == '__main__':
    unittest.main()
