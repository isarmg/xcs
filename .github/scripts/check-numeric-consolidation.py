#!/usr/bin/env python3
"""Check feature-isolated numeric consumers without mutating production manifests."""
from pathlib import Path
import json
import os
import subprocess
import tempfile
import tomllib

import runpy

ROOT = Path(__file__).resolve().parents[2]
foundation_path = runpy.run_path(str(ROOT / '.github/scripts/foundation_paths.py'))['package_path']
FEATURES = ('std', 'alloc', 'integer', 'iter', 'complex', 'bigint', 'rational', 'collection', 'serialization')
REMOVED = ('libm', 'rand', 'arbitrary', 'quickcheck', 'rkyv', 'bytecheck', 'bytemuck', 'i128', 'num-bigint-std')


def check(root=ROOT):
    source = root / 'src/foundation/core-types/numeric'
    manifest = tomllib.loads((source / 'Cargo.toml').read_text())
    assert set(manifest['features']) == {'default', *FEATURES}
    assert manifest['features']['default'] == ['std']
    for old in ('num', 'num-traits', 'num-integer', 'num-iter', 'num-complex', 'num-rational', 'num-bigint'):
        assert not (foundation_path(root, old)).exists(), old
    with tempfile.TemporaryDirectory(prefix='zxs-numeric-') as folder:
        work = Path(folder)
        (work / 'src').mkdir()
        forwarding = '\n'.join(f'{feature} = ["numeric/{feature}"]' for feature in FEATURES)
        (work / 'Cargo.toml').write_text(f'''[package]
name = "numeric-feature-check"
version = "0.0.0"
edition = "2021"
[workspace]
[features]
default = []
{forwarding}
[dependencies.numeric]
path = "{source}"
default-features = false
''')
        (work / 'src/lib.rs').write_text('''#![no_std]
pub fn zero<T: numeric::Zero>() -> T { T::zero() }
#[cfg(feature = "integer")]
pub fn gcd(a: u64, b: u64) -> u64 { numeric::integer::gcd(a, b) }
#[cfg(feature = "complex")]
pub fn complex() -> numeric::complex::Complex<i32> { numeric::complex::Complex::new(2, 3) }
#[cfg(feature = "rational")]
pub fn rational() -> numeric::rational::Ratio<i64> { numeric::rational::Ratio::new(1, 3) }
#[cfg(feature = "bigint")]
pub fn bigint() -> numeric::bigint::BigUint { numeric::bigint::BigUint::from(1u32) }
''')
        base = ['cargo', '--manifest-path', str(work / 'Cargo.toml')]
        env = os.environ.copy()
        env.setdefault('CARGO_BUILD_JOBS', '2')
        env.setdefault('CARGO_PROFILE_DEV_DEBUG', '0')
        env.setdefault('CARGO_INCREMENTAL', '0')
        def run(command, *arguments, **options):
            return subprocess.run([base[0], command, *base[1:], '--offline', *arguments], cwd=root, env=env, **options)
        run('generate-lockfile', check=True)
        metadata = json.loads(run('metadata', '--locked', '--format-version', '1', check=True, capture_output=True, text=True).stdout)
        names = {package['name'] for package in metadata['packages']}
        assert names == {'numeric-feature-check', 'numeric', 'autocfg'}, names
        identifier = next(package['id'] for package in metadata['packages'] if package['name'] == 'numeric')
        node = next(node for node in metadata['resolve']['nodes'] if node['id'] == identifier)
        assert not node['features'], node['features']
        for target in ('x86_64-unknown-linux-gnu', 'wasm32-unknown-unknown'):
            for features in ('', 'std', 'integer', 'iter', 'complex', 'rational', 'bigint', 'collection', 'collection,bigint', 'collection,bigint,serialization'):
                flags = ['--features', features] if features else []
                run('check', '--locked', '--target', target, *flags, check=True)
            run('check', '--locked', '--target', target, '--all-features', check=True)
        for feature in REMOVED:
            result = run('check', '--features', f'numeric/{feature}', capture_output=True, text=True)
            assert result.returncode and 'does not have' in result.stderr, (feature, result.stderr)
        (work / 'src/lib.rs').write_text('#![no_std]\npub use numeric::{bigint, complex, rational, iter};\n')
        result = run('check', '--locked', capture_output=True, text=True)
        assert result.returncode and 'unresolved imports' in result.stderr, result.stderr
    print('Numeric consolidation: lightweight traits, opt-in APIs, all retained features, removed-feature rejection, native/WASM verified.')


if __name__ == '__main__':
    check()
