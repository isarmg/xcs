"""Enforce tracing capability isolation and real preserved test coverage."""
import importlib.util
import json
from pathlib import Path
import subprocess
import tomllib
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('tracing_gate', ROOT / '.github/scripts/check-tracing-family.py')
GATE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GATE)

class TracingFamily(unittest.TestCase):
    def test_minimal_and_full_capabilities(self):
        self.assertEqual(len(GATE.FEATURES), 12)
        self.assertIn((), GATE.FEATURES)
        self.assertEqual(GATE.TARGETS, ('x86_64-unknown-linux-gnu', 'wasm32-unknown-unknown'))
        self.assertIn('json', GATE.FULL)
        self.assertIn('log', GATE.FULL)
        self.assertIn('env-filter', GATE.FULL)

    def test_product_does_not_enable_optional_macros_but_family_fixtures_do(self):
        server = tomllib.loads((ROOT / 'src/server/Cargo.toml').read_text())['dependencies']
        self.assertFalse(server['tracing']['default-features'])
        self.assertEqual(server['tracing']['features'], ['collector-default', 'env-filter'])
        self.assertNotIn('macros', server['http-routing'].get('features', []))
        sqlite = tomllib.loads((ROOT / 'src/foundation/data-storage/sqlite-access/Cargo.toml').read_text())
        self.assertFalse(sqlite['dependencies']['tracing']['default-features'])
        self.assertEqual(sqlite['dependencies']['tracing']['features'], ['log', 'std'])
        self.assertIn('attributes', GATE.FULL)
        spec = importlib.util.spec_from_file_location('routing_gate', ROOT / '.github/scripts/check-http-routing.py')
        routing = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(routing)
        self.assertIn(('json', 'form', 'query', 'macros'), routing.BASE_FEATURES)

    def test_native_product_macro_gate_rejects_leaks_and_missing_graphs(self):
        graph = 'storage-server v0.1.1\ntracing v0.1.44\nhttp-routing v0.8.9\n'
        with patch.object(GATE.subprocess, 'run') as cargo:
            cargo.return_value = subprocess.CompletedProcess([], 0, graph, '')
            GATE.native_product_optional_macro_closure(ROOT)
            cargo.assert_called_once_with(
                ['cargo', 'tree', '--locked', '--offline', '-p', 'storage-server',
                 '--target', 'x86_64-unknown-linux-gnu', '--edges', 'normal,build,dev',
                 '--prefix', 'none', '--format', '{p}'],
                cwd=ROOT, capture_output=True, text=True, check=True)
            for name in ('tracing-attributes', 'axum-macros'):
                with self.subTest(package=name):
                    cargo.return_value = subprocess.CompletedProcess([], 0, graph + name + ' v0.1.0\n', '')
                    with self.assertRaisesRegex(AssertionError, 'unused optional macros'):
                        GATE.native_product_optional_macro_closure(ROOT)
            cargo.return_value = subprocess.CompletedProcess([], 0, '', '')
            with self.assertRaises(AssertionError):
                GATE.native_product_optional_macro_closure(ROOT)
            cargo.side_effect = subprocess.CalledProcessError(1, ['cargo', 'tree'])
            with self.assertRaises(subprocess.CalledProcessError):
                GATE.native_product_optional_macro_closure(ROOT)

    def test_parking_provider_is_exercised(self):
        source = (ROOT / '.github/scripts/check-tracing-family.py').read_text()
        self.assertIn('parking_registry(root)', source)
        self.assertIn('test/tracing/parking.rs', source)
        self.assertIn('[\"registry\",\"parking\"]', source)

    def test_external_fixtures_pin_toolchain_before_cargo(self):
        class FirstCargo(Exception):
            pass

        def inspect(root, args):
            self.assertEqual(args[0], 'cargo')
            manifest = Path(args[args.index('--manifest-path') + 1])
            pins = [p / 'rust-toolchain.toml' for p in manifest.parents]
            pin = next(p for p in pins if p.is_file())
            self.assertEqual(pin.read_bytes(), (ROOT / 'rust-toolchain.toml').read_bytes())
            self.assertNotIn(ROOT, pin.parents)
            raise FirstCargo

        for fixture in (GATE.capabilities, GATE.retained_tests,
                        GATE.static_levels, GATE.parking_registry):
            with self.subTest(fixture=fixture.__name__):
                with patch.object(GATE, 'run', side_effect=inspect):
                    with self.assertRaises(FirstCargo):
                        fixture(ROOT)

    def test_structure(self):
        GATE.structure(ROOT)

    def test_graph_and_warning_checks_are_not_silent(self):
        source = (ROOT / '.github/scripts/check-tracing-family.py').read_text()
        self.assertIn('check-rust-diagnostics.py', source)
        self.assertIn("'pin-project-lite'", source)
        self.assertIn("'--locked'", source)
        self.assertIn("'--lib', '--tests'", source)
        self.assertIn('primitives/tests', source)
        self.assertIn('log_tracer', source)
        self.assertIn("'unresolved import' in negative.stderr", source)
        self.assertIn("'does not have' in negative.stderr", source)

    def test_runs_once_in_async_shard(self):
        inventory = json.loads((ROOT / 'test/ci/foundation-commands.json').read_text())
        seen = [suite for suite, commands in inventory.items() for command in commands if '.github/scripts/check-tracing-family.py' in command['args']]
        self.assertEqual(seen, ['foundation-async'])
        self.assertEqual((ROOT / '.github/scripts/check-foundation.sh').read_text().count('python3 .github/scripts/check-tracing-family.py'), 1)

    def test_pattern_consumers_follow_owned_modules(self):
        source = (ROOT / 'test/regex-family/consumers.rs').read_text()
        self.assertIn('tracing::pattern::Pattern', source)
        self.assertIn('tracing::collector::EnvFilter', source)
        self.assertNotIn('tracing_subscriber::', source)
        self.assertNotIn('matchers::', source)

if __name__ == '__main__':
    unittest.main()
