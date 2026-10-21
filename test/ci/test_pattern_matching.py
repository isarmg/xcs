"""The pattern gate retains real coverage and reports compiler-boundary failures."""
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location('pattern_gate', ROOT / '.github/scripts/check-pattern-matching.py')
GATE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GATE)


class PatternGateTests(unittest.TestCase):
    def test_fixture_only_rewrites_dependency_paths(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / 'root'
            source = root / 'src/foundation/regex'
            source.mkdir(parents=True)
            text = '[dependencies.memchr]\npath = "../memchr"\n[[test]]\npath = "../../../test/regex-family/retained_regressions.rs"\n'
            (source / 'Cargo.toml').write_text(text)
            for relative in ('test/foundation/regex', 'test/regex-family'):
                (root / relative).mkdir(parents=True)
            output = GATE.fixture(root, Path(directory) / 'fixture')
            content = output.read_text()
            self.assertIn(str(root / 'src/foundation/memchr'), content)
            self.assertIn('path = "../../../test/regex-family/retained_regressions.rs"', content)
            self.assertEqual((source / 'Cargo.toml').read_text(), text)
            self.assertFalse((source / 'Cargo.lock').exists())

    def test_capability_matrix_is_explicit(self):
        self.assertEqual(len(GATE.BOUNDARIES), 8)
        self.assertIn('dfa-search', GATE.WASM)
        self.assertIn('syntax', GATE.WASM)
        self.assertEqual(sum(not row[3] for row in GATE.BOUNDARIES), 4)

    def test_removed_features_are_not_empty_compatibility_names(self):
        self.assertEqual(set(GATE.REMOVED), {'arbitrary', 'pattern', 'unstable', 'use_std', 'perf-cache'})

    def test_negative_rejects_success(self):
        with patch.object(GATE.subprocess, 'run', return_value=subprocess.CompletedProcess([], 0, '')):
            with self.assertRaisesRegex(AssertionError, 'unexpected'):
                GATE.run(['cargo'], {}, False)

    def test_negative_rejects_unrelated_failure(self):
        with patch.object(GATE.subprocess, 'run', return_value=subprocess.CompletedProcess([], 1, 'disk failure')):
            with self.assertRaisesRegex(AssertionError, 'unrelated'):
                GATE.run(['cargo'], {}, False, 'could not find `syntax`')

    def test_regex_warning_is_fatal(self):
        output = '{"reason":"compiler-message","target":{"name":"regex"},"message":{"level":"warning","message":"unused"}}'
        with patch.object(GATE.subprocess, 'run', return_value=subprocess.CompletedProcess([], 0, output)):
            with self.assertRaisesRegex(AssertionError, 'regex warning'):
                GATE.run(['cargo'], {})

    def test_ignored_regression_is_named_exactly(self):
        source = (ROOT / '.github/scripts/check-pattern-matching.py').read_text()
        self.assertIn('"regression_fuzz::fuzz1", "--", "--ignored", "--exact"', source)
        self.assertIn('"--all-features", "--doc"', source)
        self.assertNotIn('/workspace/', source)


if __name__ == '__main__':
    unittest.main()
