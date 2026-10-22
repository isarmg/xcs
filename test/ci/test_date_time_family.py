"""Keep the date/time gate on real owned sources and truthful native/WASM coverage."""
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import tomllib
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('date_time_gate', ROOT / '.github/scripts/check-date-time-family.py')
GATE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GATE)


class DateTimeFamilyTests(unittest.TestCase):
    def test_structure_and_removed_packages(self):
        GATE.structure(ROOT)
        self.assertEqual(set(GATE.ABSORBED), {'time', 'time-core', 'deranged', 'powerfmt', 'num-conv'})
        self.assertEqual(set(GATE.PRIVATE), {'calendar_core', 'ranged', 'display', 'num_conv'})

    def test_matrix_preserves_allocation_free_and_wasm_capabilities(self):
        self.assertEqual(GATE.TARGETS, ('x86_64-unknown-linux-gnu', 'wasm32-unknown-unknown'))
        for features in ((), ('alloc',), ('parsing',), ('serialization',), ('wasm-bindgen',)):
            self.assertIn(features, GATE.MATRIX)
        self.assertIn(GATE.FEATURES, GATE.MATRIX)
        self.assertNotIn('local-offset', GATE.FEATURES)
        self.assertEqual(set(GATE.REMOVED), {'macros', 'quickcheck', 'rand', 'rand08', 'rand09', 'local-offset'})
        self.assertIn(('large-dates',), GATE.TEST_PROFILES)
        self.assertIn(('serialization',), GATE.SERIALIZATION_PROFILES)
        self.assertIn(('serialization-well-known',), GATE.SERIALIZATION_PROFILES)

    def test_isolated_probes_pin_toolchain_and_use_real_local_packages(self):
        with tempfile.TemporaryDirectory() as directory:
            for fixture in ('contracts', 'serialization', 'tracing'):
                with self.subTest(fixture=fixture):
                    work = GATE.prepare(ROOT, Path(directory), fixture)
                    self.assertEqual((work / 'rust-toolchain.toml').read_bytes(), (ROOT / 'rust-toolchain.toml').read_bytes())
                    self.assertTrue((work / 'src/lib.rs').is_file())
                    manifest = tomllib.loads((work / 'Cargo.toml').read_text())
                    self.assertIn('workspace', manifest)
                    for name, dependency in manifest['dependencies'].items():
                        package = tomllib.loads((Path(dependency['path']) / 'Cargo.toml').read_text())
                        self.assertEqual(package['package']['name'], name)
                        self.assertTrue(set(dependency.get('features', [])) <= package.get('features', {}).keys())
                    self.assertTrue(Path(manifest['test'][0]['path']).is_relative_to(ROOT / 'test/date-time'))
                    self.assertFalse(manifest['dependencies']['date-time']['default-features'])
                    if fixture == 'contracts':
                        self.assertIn('#![no_std]', (work / 'src/lib.rs').read_text())
                    if fixture == 'tracing':
                        self.assertEqual(manifest['dependencies']['tracing']['features'], ['time', 'fmt', 'json'])

    def test_rejection_checks_fail_closed_on_unexpected_success(self):
        with tempfile.TemporaryDirectory() as directory:
            work = GATE.prepare(ROOT, Path(directory), 'contracts')
            with patch.object(GATE, 'run', return_value=subprocess.CompletedProcess([], 0, '', '')):
                with self.assertRaises(AssertionError):
                    GATE.reject_features_and_private_api(work, {})

    def test_warning_checks_and_real_compiler_failure_codes_are_retained(self):
        source = (ROOT / '.github/scripts/check-date-time-family.py').read_text()
        for code in ('error[E0603]', 'error[E0599]', 'error[E0432]', 'error[E0080]'):
            self.assertIn(code, source)
        self.assertIn("'--locked'", source)
        self.assertIn("'-j2'", source)
        self.assertIn("'check', '--tests', '--target', TARGETS[1]", source)
        self.assertNotIn("'test', '--tests', '--target', TARGETS[1]", source)
        self.assertIn('matrix_environment', source)
        with tempfile.TemporaryDirectory() as directory:
            result = subprocess.CompletedProcess([], 0, '', 'warning: untested code\n')
            with patch.object(GATE.subprocess, 'run', return_value=result):
                with self.assertRaises(AssertionError):
                    GATE.run(Path(directory), {}, 'check')

    def test_serialization_and_tracing_are_behavioral_consumers(self):
        serialization = (ROOT / 'test/date-time/serialization.rs').read_text()
        self.assertIn('json_codec::to_string', serialization)
        self.assertIn('json_codec::from_str', serialization)
        self.assertIn('serialization-human-readable', serialization)
        self.assertIn('date_time::data_serialization::rfc3339', serialization)
        tracing = (ROOT / 'test/date-time/tracing.rs').read_text()
        self.assertIn('UtcTime::rfc_3339()', tracing)
        self.assertIn('OffsetTime::new(offset, Rfc3339)', tracing)
        self.assertIn('UtcOffset::from_hms(5, 30, 0)', tracing)
        self.assertIn('tracing::info!', tracing)
        self.assertIn('event["timestamp"]', tracing)

    def test_runs_once_in_data_shard(self):
        inventory = json.loads((ROOT / 'test/ci/foundation-commands.json').read_text())
        suites = [suite for suite, commands in inventory.items() for command in commands
                  if '.github/scripts/check-date-time-family.py' in command['args']]
        self.assertEqual(suites, ['foundation-data'])
        self.assertEqual((ROOT / '.github/scripts/check-foundation.sh').read_text().count('python3 .github/scripts/check-date-time-family.py'), 1)


if __name__ == '__main__':
    unittest.main()
