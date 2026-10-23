"""Logging matrix gate must keep behavioral and supported-target coverage."""
import importlib.util
import json
from pathlib import Path
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('logging_gate', ROOT / '.github/scripts/check-logging.py')
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)


class LoggingGate(unittest.TestCase):
    def metadata(self, args, **kwargs):
        features = args[args.index('--features') + 1] if '--features' in args else ''
        names = ['logging', 'log']
        if 'regex' in features:
            names.append('regex')
        if 'color' in features:
            names.append('terminal-style')
        return json.dumps({'packages': [{'id': n, 'name': n} for n in names],
                           'resolve': {'nodes': [{'id': n} for n in names]}}).encode()

    def test_native_behavior_and_wasm_checks_for_each_feature_set(self):
        with patch.object(gate.subprocess, 'check_output', side_effect=self.metadata), \
                patch.object(gate.subprocess, 'run') as run:
            gate.check()
        calls = [call.args[0] for call in run.call_args_list]
        self.assertEqual(len(calls), 12)
        self.assertEqual(sum(args[1] == 'test' for args in calls), 6)
        self.assertEqual(sum('wasm32-unknown-unknown' in args for args in calls), 6)
        self.assertTrue(all('--no-default-features' in args for args in calls))
        self.assertTrue(all(call.kwargs['check'] for call in run.call_args_list))

    def test_rejects_old_package_edges(self):
        def stale(args, **kwargs):
            graph = json.loads(self.metadata(args))
            graph['packages'].append({'id': 'old', 'name': 'env_filter'})
            graph['resolve']['nodes'].append({'id': 'old'})
            return json.dumps(graph).encode()
        with patch.object(gate.subprocess, 'check_output', side_effect=stale), \
                patch.object(gate.subprocess, 'run') as run:
            with self.assertRaises(AssertionError):
                gate.check()
            run.assert_not_called()

    def test_obsolete_alias_must_fail_at_cargo_feature_resolution(self):
        from subprocess import CompletedProcess
        good = CompletedProcess([], 101, stderr="package logging does not contain this feature: unstable-kv")
        with patch.object(gate.subprocess, 'run', return_value=good):
            gate.check_removed_alias()
        for bad in (CompletedProcess([], 0, stderr=''),
                    CompletedProcess([], 101, stderr='compiler failed')):
            with patch.object(gate.subprocess, 'run', return_value=bad):
                with self.assertRaises(AssertionError):
                    gate.check_removed_alias()

    def test_upstream_readme_examples_are_preserved_with_only_crate_rename(self):
        import re
        original = (ROOT / 'docs/upstream/env_logger-0.11.11/README.md').read_text()
        adapted = ''.join(line.removeprefix('//!').removeprefix(' ') + '\n' for line in (ROOT / 'test/logging/readme_doctests.rs').read_text().splitlines())
        blocks = lambda text: re.findall(r'```rust\n(.*?)```', text, re.S)
        self.assertEqual(blocks(adapted), [b.replace('env_logger::', 'logging::') for b in blocks(original)])
        self.assertEqual(len(blocks(adapted)), 3)

    def test_manifest_exposes_only_canonical_logging_features(self):
        import tomllib
        manifest = tomllib.loads(gate.MANIFEST.read_text())
        self.assertEqual(manifest['package']['name'], 'logging')
        self.assertEqual(set(manifest['features']), {'auto-color', 'color', 'default', 'kv', 'regex'})
        self.assertNotIn('env_filter', manifest['dependencies'])
        self.assertEqual(manifest['dependencies']['terminal-style']['path'], '../../tooling/terminal-style')
        self.assertEqual(manifest['dependencies']['regex']['features'], ['matcher', 'perf', 'std'])
