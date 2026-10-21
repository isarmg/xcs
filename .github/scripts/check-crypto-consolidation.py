#!/usr/bin/env python3
"""Offline BLAKE2/Argon2 consolidation regression gate; no production mutation."""
from pathlib import Path
import os
import runpy
import subprocess
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[2]


def manifest(root):
    foundation = root / 'src/foundation'
    return f'''[package]
name = "crypto-consolidation-regression"
version = "0.0.0"
edition = "2021"
[workspace]
[features]
default = ["std", "password-hash", "rand"]
std = ["alloc", "blake2/std", "argon2/std"]
alloc = ["blake2/alloc", "argon2/alloc"]
reset = ["blake2/reset"]
size_opt = ["blake2/size_opt"]
password-hash = ["argon2/password-hash"]
rand = ["argon2/rand"]
[dependencies]
blake2 = {{path = "{foundation / 'blake2'}", default-features = false}}
argon2 = {{path = "{foundation / 'argon2'}", default-features = false}}
password-hash = {{path = "{foundation / 'password-hash'}", default-features = false}}
base64ct = {{path = "{foundation / 'base64ct'}", default-features = false}}
'''


def check_shared_stack(root, metadata):
    """Fail if any reachable package restores the duplicate array/digest stack."""
    names = [p['name'] for p in metadata['packages']]
    for name in ('digest', 'block-buffer', 'crypto-common'):
        if names.count(name) != 1:
            raise AssertionError(f'exactly one {name} implementation is required')
    if 'generic-array' in names:
        raise AssertionError('generic-array returned to the crypto closure')
    for relative in ('digest-0.10.7', 'block-buffer-0.10.4', 'crypto-common-0.1.7', 'generic-array'):
        if (root / 'src/foundation' / relative).exists():
            raise AssertionError(f'removed standalone crypto package returned: {relative}')


def check(root):
    environment = runpy.run_path(str(root / '.github/scripts/backend_environment.py'))['matrix_environment']()
    generate = runpy.run_path(str(root / 'test/ci/crypto_vectors.py'))['generate']
    with tempfile.TemporaryDirectory(prefix='zxs-crypto-consolidation-') as directory:
        work = Path(directory)
        (work / 'src').mkdir()
        (work / 'src/lib.rs').write_text('''#![no_std]
pub use blake2::{Blake2b512, Blake2bVar, VariableOutput};
pub use argon2::Argon2;
''')
        (work / 'Cargo.toml').write_text(manifest(root))
        counts = generate(root, work)
        (work / 'tests/capabilities.rs').write_bytes((root / 'test/ci/fixtures/crypto-capabilities.rs').read_bytes())
        base = ['cargo', '--manifest-path', str(work / 'Cargo.toml')]
        def cargo(command, *arguments):
            subprocess.run([base[0], command, *base[1:], '--offline', *arguments], check=True, cwd=root, env=environment)
        cargo('generate-lockfile')
        import json
        result = subprocess.run([base[0], 'metadata', *base[1:], '--locked', '--offline', '--format-version', '1'], check=True, capture_output=True, text=True, cwd=root, env=environment)
        check_shared_stack(root, json.loads(result.stdout))
        for flags in ([], ['--features', 'reset'], ['--no-default-features'], ['--no-default-features', '--features', 'reset'], ['--no-default-features', '--features', 'alloc'], ['--features', 'reset,size_opt']):
            cargo('test', '--locked', '--release', *flags, '--test', 'vectors', '--test', 'capabilities')
        cargo('test', '--locked', '--release', '--test', 'kat', '--test', 'phc_strings')
        for target in ('x86_64-unknown-linux-gnu', 'wasm32-unknown-unknown'):
            for flags in ([], ['--features', 'reset'], ['--features', 'alloc']):
                cargo('check', '--locked', '--no-default-features', '--target', target, *flags)
        print(f"Crypto consolidation verified: {counts['upstream_vectors']} preserved + {counts['independent_vectors']} independent vectors, capability matrix, Argon2 KAT/PHC and native/WASM no_std.")


if __name__ == '__main__':
    check(ROOT)
