#!/usr/bin/env python3
"""Gate unified float formatting, form round-trips, and real HTTP consumers."""
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]


def main():
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
            manifest.append(f'{name}={{path={json.dumps(str(ROOT / "src/foundation" / name))},default-features=false,features={json.dumps(features)}}}')
        manifest.extend(["[target.'cfg(not(target_arch = \"wasm32\"))'.dependencies]",
                         f'reqwest={{path={json.dumps(str(ROOT / "src/foundation/reqwest"))},default-features=false}}'])
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
