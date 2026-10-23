#!/usr/bin/env python3
"""Gate unified float formatting, form round-trips, and real HTTP consumers."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile

from retired_sources import restore

import runpy

ROOT = Path(__file__).resolve().parents[2]
foundation_path = runpy.run_path(str(ROOT / '.github/scripts/foundation_paths.py'))['package_path']



def differential():
    """Replay the full historical audit without Git history or active ryu code."""
    with tempfile.TemporaryDirectory(prefix='form-float-oracle-') as temporary:
        work = Path(temporary)
        restore(ROOT, work, ['ryu'])
        manifest = ['[package]', 'name="form-float-differential"', 'version="0.0.0"',
                    'edition="2021"', '[workspace]', '[[bin]]', 'name="differential"',
                    'path=' + json.dumps(str(ROOT / 'test/architecture/form-floats/differential.rs')),
                    '[dependencies]']
        for name, path in [('ryu', work / 'src/foundation/ryu'),
                           ('zmij', ROOT / 'src/foundation/core-types/zmij'),
                           ('form-codec', ROOT / 'src/foundation/data-storage/form-codec')]:
            manifest.append(f'{name}={{path={json.dumps(str(path))}}}')
        (work / 'Cargo.toml').write_text('\n'.join(manifest) + '\n')
        (work / 'rust-toolchain.toml').write_bytes((ROOT / 'rust-toolchain.toml').read_bytes())
        env = dict(os.environ, CARGO_BUILD_JOBS='1', CARGO_INCREMENTAL='0',
                   CARGO_PROFILE_RELEASE_DEBUG='0')
        env['CARGO_TARGET_DIR'] = str(work / 'target')
        subprocess.run(['cargo', 'run', '--release', '--offline', '--manifest-path',
                        str(work / 'Cargo.toml')], check=True, cwd=ROOT, env=env)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--differential', action='store_true',
                        help='replay the complete archived-oracle audit instead of the fast consumer gate')
    if parser.parse_args().differential:
        differential()
        return
    os.environ.setdefault('CARGO_BUILD_JOBS', '1')
    os.environ.setdefault('CARGO_PROFILE_DEV_DEBUG', '0')
    os.environ.setdefault('CARGO_INCREMENTAL', '0')
    with tempfile.TemporaryDirectory(prefix='form-floats-') as temporary:
        work = Path(temporary)
        manifest = ['[package]', 'name="form-floats-regression"', 'version="0.0.0"',
                    'edition="2021"', '[workspace]', '[dependencies]']
        for name, features in [('form-codec', []), ('data-serialization', ['derive']),
                               ('serialization-derive', []), ('http-routing', ['form', 'query']),
                               ('http-body', ['utilities'])]:
            manifest.append(f'{name}={{path={json.dumps(str(foundation_path(ROOT, name)))},default-features=false,features={json.dumps(features)}}}')
        manifest.extend(["[target.'cfg(not(target_arch = \"wasm32\"))'.dependencies]",
                         f'reqwest={{path={json.dumps(str(ROOT / "src/foundation/networking/reqwest"))},default-features=false}}'])
        for name, path in [('behavior', 'test/architecture/form-floats/behavior.rs'),
                           ('serialize', 'test/foundation/form-codec/tests/test_serialize.rs'),
                           ('deserialize', 'test/foundation/form-codec/tests/test_deserialize.rs')]:
            manifest.extend(['[[test]]', f'name={json.dumps(name)}', f'path={json.dumps(str(ROOT / path))}'])
        (work / 'Cargo.toml').write_text('\n'.join(manifest) + '\n')
        (work / 'rust-toolchain.toml').write_bytes((ROOT / 'rust-toolchain.toml').read_bytes())
        def run(*args):
            subprocess.run(['cargo', *args, '--offline', '--manifest-path', str(work / 'Cargo.toml')], check=True, cwd=ROOT)
        run('generate-lockfile')
        metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--offline', '--format-version=1', '--manifest-path', str(work / 'Cargo.toml')], cwd=ROOT))
        names = {p['name'] for p in metadata['packages']}
        if 'zmij' not in names or 'ryu' in names:
            raise ValueError(f'form float dependency graph drift: {sorted(names)}')
        run('test', '--locked', '--target', 'x86_64-unknown-linux-gnu')
        run('check', '--locked', '--tests', '--target', 'wasm32-unknown-unknown')
        print('form-floats: canonical wire spelling, bit round-trips, form suites, HTTP consumers and native/WASM checks passed; no ryu dependency')


if __name__ == '__main__':
    main()
