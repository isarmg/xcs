"""Native C fixture builds must retain the repository Rust toolchain."""
import importlib.util
from pathlib import Path
import unittest
import tempfile
import subprocess
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

    def test_standalone_unit_tests_never_create_a_source_tree_lock(self):
        source = ROOT / 'src/foundation/tooling/cc'
        source_lock = source / 'Cargo.lock'
        before = source_lock.read_bytes() if source_lock.exists() else None
        for fail in (False, True):
            with self.subTest(fail=fail), tempfile.TemporaryDirectory() as temporary:
                work = Path(temporary)

                def cargo_test(command, *, cwd, env, check):
                    self.assertEqual(command[:3], ['cargo', 'test', '--offline'])
                    manifest = Path(command[-1])
                    self.assertTrue(manifest.is_relative_to(work))
                    self.assertFalse(manifest.is_relative_to(ROOT))
                    self.assertIn('[workspace]', (cwd / 'Cargo.toml').read_text())
                    self.assertIn('src/foundation/tooling/cc', (cwd / 'Cargo.toml').read_text())
                    self.assertEqual(manifest.read_bytes(), (source / 'Cargo.toml').read_bytes())
                    self.assertEqual((manifest.parent / 'src/lib.rs').read_bytes(),
                                     (source / 'src/lib.rs').read_bytes())
                    self.assertEqual((cwd / 'rust-toolchain.toml').read_bytes(),
                                     (ROOT / 'rust-toolchain.toml').read_bytes())
                    self.assertEqual((manifest.parent / 'src/target/parser.rs').read_bytes(),
                                     (source / 'src/target/parser.rs').read_bytes())
                    self.assertTrue((cwd / 'test/foundation/cc/unit/src/flags/tests_425.rs').is_file())
                    self.assertTrue((manifest.parent.parent / 'shell-tokenizer/Cargo.toml').is_file())
                    manifest.with_name('Cargo.lock').write_text('temporary resolution')
                    if fail:
                        raise subprocess.CalledProcessError(1, command)

                with patch.object(gate.subprocess, 'run', side_effect=cargo_test) as cargo:
                    if fail:
                        with self.assertRaises(subprocess.CalledProcessError):
                            gate.check_standalone_cc(work, {})
                    else:
                        gate.check_standalone_cc(work, {})
                cargo.assert_called_once()
                after = source_lock.read_bytes() if source_lock.exists() else None
                self.assertEqual(after, before)


if __name__ == '__main__':
    unittest.main()
