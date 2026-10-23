#!/usr/bin/env python3
"""Regression coverage for the upgraded source's retained boundary tests and CI gate."""
from pathlib import Path
import json
import runpy
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / '.github/scripts/check-ctutils.py'


class CtutilsUpgradeTests(unittest.TestCase):
    def test_runner_covers_features_and_targets_without_nested_workspace(self):
        runner = runpy.run_path(str(SCRIPT))
        with mock.patch('subprocess.run') as run:
            runner['check']()
        calls = [call.args[0] for call in run.call_args_list]
        compiles = [cmd for cmd in calls if cmd[0] == 'rustc']
        self.assertEqual(len(compiles), 8)
        ctutils = [cmd for cmd in compiles if cmd[cmd.index('--crate-name') + 1] == 'ctutils']
        self.assertEqual(len(ctutils), 4)
        for cmd in ctutils:
            self.assertIn('-Dwarnings', cmd)
        self.assertEqual(sum('--test' in cmd for cmd in ctutils), 2)
        self.assertEqual(sum('wasm32-unknown-unknown' in cmd for cmd in ctutils), 2)
        self.assertEqual(sum('feature="subtle"' in cmd for cmd in ctutils), 2)
        self.assertEqual(len(calls) - len(compiles), 2)

    def test_release_crypto_shard_keeps_regression(self):
        command = 'python3 .github/scripts/check-ctutils.py'
        self.assertIn(command, (ROOT / '.github/scripts/check-foundation.sh').read_text())
        shard = json.loads((ROOT / 'test/ci/foundation-commands.json').read_text())['foundation-crypto']
        self.assertIn({'tool': 'python3', 'args': ['.github/scripts/check-ctutils.py']}, shard)


if __name__ == '__main__':
    unittest.main()
