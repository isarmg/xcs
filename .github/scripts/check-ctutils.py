#!/usr/bin/env python3
"""Offline ctutils unit regressions and wasm32 compile check, not a timing proof."""
from pathlib import Path
import runpy
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]


def check(root=ROOT):
    environment = runpy.run_path(str(root / '.github/scripts/backend_environment.py'))['matrix_environment']()
    foundation = root / 'src/foundation/cryptography'
    with tempfile.TemporaryDirectory(prefix='zxs-ctutils-') as temporary:
        output = Path(temporary)
        for target in ('x86_64-unknown-linux-gnu', 'wasm32-unknown-unknown'):
            directory = output / target
            directory.mkdir()
            for name, edition in (('cmov', '2024'), ('subtle', '2018')):
                env = dict(environment, CARGO_MANIFEST_DIR=str(foundation / name))
                subprocess.run(['rustc', '--crate-name', name, '--edition=' + edition,
                                '--crate-type=rlib', '--target', target, '-Dwarnings',
                                str(foundation / name / 'src/lib.rs'), '-o', str(directory / ('lib' + name + '.rlib'))],
                               cwd=root, env=env, check=True)
            for features in ((), ('alloc', 'subtle')):
                env = dict(environment, CARGO_MANIFEST_DIR=str(foundation / 'ctutils'))
                command = ['rustc', '--crate-name', 'ctutils', '--edition=2024', '--target', target,
                           '-Dwarnings', str(foundation / 'ctutils/src/lib.rs'),
                           '-L', 'dependency=' + str(directory), '--extern', 'cmov=' + str(directory / 'libcmov.rlib')]
                for feature in features:
                    command += ['--cfg', 'feature="' + feature + '"']
                if 'subtle' in features:
                    command += ['--extern', 'subtle=' + str(directory / 'libsubtle.rlib')]
                artifact = directory / ('tests-all' if features else 'tests-default')
                if target == 'x86_64-unknown-linux-gnu':
                    command += ['--test', '-o', str(artifact)]
                    subprocess.run(command, cwd=root, env=env, check=True)
                    subprocess.run([str(artifact)], cwd=root, env=env, check=True)
                else:
                    subprocess.run(command + ['--crate-type=rlib', '-o', str(artifact) + '.rlib'],
                                   cwd=root, env=env, check=True)
    print('ctutils default/all-feature native unit tests and wasm32 compilation passed; no timing-proof claim.')


if __name__ == '__main__':
    check()
