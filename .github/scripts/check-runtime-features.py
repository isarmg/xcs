#!/usr/bin/env python3
"""Exercise runtime/adapter boundaries without touching the product lockfile."""
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
FEATURES = ((), ('rt', 'sync', 'time'), ('adapters',), ('adapters-io',),
            ('adapters-time',), ('adapters-rt',), ('adapters-compat',),
            ('adapters-io-util',), ('adapters-codec',), ('adapters-net',),
            ('adapters-tracing',), ('full', 'adapters-full'))


def check(root=ROOT):
    root = Path(root).resolve()
    with tempfile.TemporaryDirectory(prefix='runtime-features-') as temporary:
        work = Path(temporary)
        (work / 'src').mkdir()
        (work / 'src/lib.rs').write_text('// Boundary compilation probe.\n')
        (work / 'Cargo.toml').write_text('''[package]
name = "runtime-feature-boundary"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
tokio = { path = ''' + json.dumps(str(root / 'src/foundation/concurrency/tokio')) + ''', default-features = false }
''')
        environment = dict(os.environ, CARGO_BUILD_JOBS='1', CARGO_PROFILE_DEV_DEBUG='0', CARGO_INCREMENTAL='0')
        environment.setdefault('CARGO_TARGET_DIR', str(root / 'target/runtime-join-map-consumer'))
        def cargo(command, *arguments, capture=False):
            return subprocess.run(['cargo', command, '--manifest-path', str(work / 'Cargo.toml'),
                                   '--offline', *arguments], cwd=root, env=environment,
                                  check=True, timeout=300, capture_output=capture, text=True)
        cargo('generate-lockfile')
        for target in ('x86_64-unknown-linux-gnu', 'wasm32-unknown-unknown'):
            for features in FEATURES:
                if target.startswith('wasm') and features in (('adapters-net',), ('full', 'adapters-full')):
                    continue  # Core net/process/signal and multi-thread runtime do not support browser WASM.
                flags = ['--features', ','.join('tokio/' + feature for feature in features)] if features else []
                cargo('check', '--locked', '--target', target, *flags)
                if features in ((), ('rt', 'sync', 'time')):
                    tree = cargo('tree', '--locked', '--target', target, '-e', 'normal', '--prefix', 'none', *flags, capture=True).stdout
                    for forbidden in ('futures ', 'bytes ', 'hash-table ', 'tokio-util '):
                        assert forbidden not in tree, (features, target, tree)
        cargo('check', '--locked', '--target', 'wasm32-unknown-unknown', '--features',
              'tokio/rt,tokio/sync,tokio/time,tokio/macros,tokio/adapters-io-util,tokio/adapters-time,tokio/adapters-join-map')

        # This optional consumer owns retained internal file-body streaming, not Web UI.
        (work / 'Cargo.toml').write_text('[package]\nname="runtime-file-consumer"\nversion="0.0.0"\nedition="2021"\n[workspace]\n'
            '[dependencies]\ntower-http={path=' + json.dumps(str(root / 'src/foundation/networking/tower-http')) +
            ',default-features=false,features=["fs"]}\n')
        (work / 'src/lib.rs').write_text('pub use tower_http::services::ServeDir;\n')
        cargo('generate-lockfile')
        cargo('check', '--locked', '--target', 'x86_64-unknown-linux-gnu')


if __name__ == '__main__':
    check()
