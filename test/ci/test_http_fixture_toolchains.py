"""Isolated HTTP fixture commands must select the repository Rust toolchain."""
import importlib.util
import os
from pathlib import Path
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]


class FirstCargoCommand(Exception):
    """Stop before invoking any compiler in this portable wiring test."""


class HttpFixtureToolchains(unittest.TestCase):
    def test_every_installed_http_gate_inherits_toolchain_before_cargo(self):
        checked = 0
        for family in ('service-stack', 'http-routing', 'http-transport'):
            path = ROOT / '.github/scripts' / f'check-{family}.py'
            if not path.exists():
                continue
            with self.subTest(family=family):
                spec = importlib.util.spec_from_file_location('http_gate', path)
                module = importlib.util.module_from_spec(spec)
                spec.loader.exec_module(module)
                calls = []

                def first_command(work, *args):
                    self.assertEqual(
                        (Path(work) / 'rust-toolchain.toml').read_bytes(),
                        (ROOT / 'rust-toolchain.toml').read_bytes(),
                    )
                    calls.append(args)
                    raise FirstCargoCommand

                with mock.patch.dict(os.environ), mock.patch.object(module, 'run', first_command):
                    with self.assertRaises(FirstCargoCommand):
                        module.main()
                self.assertEqual(len(calls), 1)
                checked += 1
        self.assertGreaterEqual(checked, 2)


if __name__ == '__main__':
    unittest.main()
