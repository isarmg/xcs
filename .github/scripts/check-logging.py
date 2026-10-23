#!/usr/bin/env python3
"""Exercise logging filters, formatting, terminal adaptation and browser compilation."""
if not __debug__:
    raise RuntimeError('Validation requires assertions; do not use Python optimization (-O/-OO or PYTHONOPTIMIZE).')

import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / 'src/foundation/diagnostics/logging/Cargo.toml'


def check():
    env = dict(os.environ, CARGO_BUILD_JOBS=os.environ.get('CARGO_BUILD_JOBS', '1'))
    for features in ('', 'regex', 'color', 'kv', 'auto-color,regex', 'auto-color,regex,kv'):
        args = ['--offline', '--manifest-path', str(MANIFEST), '--no-default-features']
        if features:
            args += ['--features', features]
        graph = json.loads(subprocess.check_output(
            ['cargo', 'metadata', *args, '--format-version', '1'], cwd=ROOT, env=env))
        names = {p['id']: p['name'] for p in graph['packages']}
        active = {names[n['id']] for n in graph['resolve']['nodes']}
        assert not active & {'env_logger', 'env_filter'}, active
        assert ('regex' in active) == ('regex' in features), active
        assert ('terminal-style' in active) == ('color' in features), active
        subprocess.run(['cargo', 'test', *args], cwd=ROOT, env=env, check=True)
        subprocess.run(['cargo', 'check', *args, '--target', 'wasm32-unknown-unknown'],
                       cwd=ROOT, env=env, check=True)


def check_removed_alias():
    result = subprocess.run(
        ['cargo', 'check', '--offline', '--manifest-path', str(MANIFEST),
         '--features', 'unstable-kv'], cwd=ROOT, capture_output=True, text=True)
    assert result.returncode and 'does not contain this feature' in result.stderr and 'unstable-kv' in result.stderr, result.stderr


def main():
    lock = MANIFEST.with_name('Cargo.lock')
    existed = lock.exists()
    try:
        check()
        check_removed_alias()
    finally:
        if not existed and lock.exists():
            lock.unlink()


if __name__ == '__main__':
    main()
