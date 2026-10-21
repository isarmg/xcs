"""Keep TOML checks isolated, locked and connected to the maintained source."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock
ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('toml_gate', ROOT / '.github/scripts/check-toml-consolidation.py')
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)


class TomlConsolidation(unittest.TestCase):
    def test_commands_lock_existing_resolution(self):
        self.assertEqual(gate.command('test', '-p', 'toml'),
                         ['cargo', 'test', '--locked', '--offline', '-p', 'toml'])

    def test_matrix_keeps_both_targets_and_minimal_features(self):
        self.assertEqual(len(gate.FEATURE_SETS), 15)
        self.assertIn((), gate.FEATURE_SETS)
        self.assertEqual(gate.TARGETS, ('x86_64-unknown-linux-gnu', 'wasm32-unknown-unknown'))

    def test_fixture_disables_defaults_and_generates_only_temporary_lock(self):
        with tempfile.TemporaryDirectory() as temp:
            directory = Path(temp)
            with mock.patch.object(gate.subprocess, 'run') as run:
                manifest, _ = gate.fixture(ROOT, directory, ('write',))
            self.assertIn('default-features=false', manifest.read_text())
            self.assertIn('features=["write"]', manifest.read_text())
            run.assert_called_once_with(['cargo', 'generate-lockfile', '--offline', '--manifest-path', str(manifest)], cwd=ROOT, check=True)

    def test_feature_builds_use_warning_rejecting_checker(self):
        source = (ROOT / '.github/scripts/check-toml-consolidation.py').read_text()
        self.assertIn('check-rust-diagnostics.py', source)
        self.assertIn('--message-format=json', source)

    def test_retained_serialization_and_foundation_consumers_use_new_modules(self):
        standard = (ROOT / 'test/ci/fixtures/serialization-family.rs').read_text()
        foundation = (ROOT / 'test/ci/fixtures/foundation-consumers.rs').read_text()
        self.assertIn('toml::span::__unstable', standard)
        self.assertNotIn('source_spanned_value::', standard)
        self.assertIn('toml::editor::DocumentMut', foundation)
        self.assertNotIn('toml_edit::', foundation)

    def test_current_origin_inventory(self):
        gate.check_structure(ROOT)

    def test_origin_loss_is_rejected(self):
        with mock.patch.object(gate.json, 'loads', return_value={'packages': {}, 'merges': []}):
            with self.assertRaisesRegex(AssertionError, 'original source identity'):
                gate.check_structure(ROOT)


if __name__ == '__main__':
    unittest.main()
