"""Minimal core features and buffer safety checks using temporary consumers only."""
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

FEATURES = ('', 'alloc', 'block-api', 'mac', 'oid', 'zeroize',
            'dev', 'alloc,block-api,mac,oid,zeroize,dev')


def buffer_fixture(root):
    """Preserve the eleven upstream cases, replacing only hex literal syntax."""
    source = (root / 'test/foundation/cryptographic-core/buffer/tests/mod.rs').read_text()
    if len(source.encode()) > 32768:
        raise ValueError('unexpected buffer fixture size')
    source = source.replace('use hex_literal::hex;', '')
    pattern = r'hex!\(\s*((?:"[0-9a-fA-F\s]+"\s*)+)\)'
    def literal(match):
        value = bytes.fromhex(match[1].replace('"', ''))
        if len(value) > 128:
            raise ValueError('unexpected buffer literal size')
        return '[' + ','.join(str(byte) + 'u8' for byte in value) + ']'
    source, count = re.subn(pattern, literal, source)
    if count != 10 or 'hex!' in source:
        raise ValueError('buffer fixture syntax changed; review generator')
    source = source.replace('//! `block-buffer` tests.', '// Preserved buffer tests.')
    return '#![cfg(feature="block-api")]\n' + source


def manifest(core):
    return '''[package]
name = "cryptographic-core-boundary-check"
version = "0.0.0"
edition = "2024"
[workspace]
[features]
default = []
dev = ["cryptographic-core/dev"]
alloc = ["cryptographic-core/alloc"]
block-api = ["cryptographic-core/block-api"]
mac = ["cryptographic-core/mac"]
oid = ["cryptographic-core/oid"]
zeroize = ["cryptographic-core/zeroize"]
[dependencies]
cryptographic-core = {path = "''' + core.as_posix() + '''", default-features = false}
'''


def check(root, environment):
    """Never mutate maintained sources, including for negative lint tests."""
    core = root / 'src/foundation/cryptographic-core'
    with tempfile.TemporaryDirectory(prefix='zxs-crypto-core-') as temporary:
        work = Path(temporary)
        (work / 'src').mkdir()
        (work / 'tests').mkdir()
        (work / 'Cargo.toml').write_text(manifest(core))
        probe = '#![no_std]\npub use cryptographic_core::{Digest,common_types::KeySizeUser};\n'
        (work / 'src/lib.rs').write_text(probe)
        (work / 'tests/buffer.rs').write_text(buffer_fixture(root))
        dummy = (root / 'test/foundation/cryptographic-core/hash/tests/dummy_fixed.rs').read_text()
        dummy = dummy.replace('use zeroize::Zeroize;', 'use cryptographic_core::zeroize::Zeroize;').replace('zeroize::ZeroizeOnDrop', 'cryptographic_core::zeroize::ZeroizeOnDrop')
        (work / 'tests/data').mkdir()
        shutil.copy2(root / 'test/foundation/cryptographic-core/hash/tests/data/fixed_hash_serialization.bin',
                     work / 'tests/data/fixed_hash_serialization.bin')
        dummy += (root / 'test/ci/fixtures/crypto-core-dev.rs').read_text()
        (work / 'tests/dummy_fixed.rs').write_text(dummy)
        (work / 'tests/zeroize.rs').write_bytes((root / 'test/ci/fixtures/crypto-core-zeroize.rs').read_bytes())
        def cargo(command, *arguments, capture=False):
            result = subprocess.run(['cargo', command, '--manifest-path', str(work / 'Cargo.toml'),
                                     '--offline', *arguments], cwd=root, env=environment,
                                    capture_output=capture, text=capture, check=not capture)
            return result
        cargo('generate-lockfile')
        for target in ('x86_64-unknown-linux-gnu', 'wasm32-unknown-unknown'):
            for features in FEATURES:
                flags = ['--features', features] if features else []
                cargo('check', '--locked', '--no-default-features', '--target', target, *flags)
        for target in ('x86_64-unknown-linux-gnu', 'wasm32-unknown-unknown'):
            cargo('check', '--locked', '--all-features', '--target', target)
        for removed in ('rand_core', 'getrandom'):
            result = cargo('check', '--features', 'cryptographic-core/' + removed, capture=True)
            if not result.returncode or 'does not have' not in result.stderr or removed not in result.stderr:
                raise AssertionError(f'unsupported feature was not rejected: {result.stderr}')
        for features in ('block-api,dev', 'block-api,oid,zeroize,dev'):
            cargo('test', '--locked', '--features', features, '--test', 'buffer', '--test', 'zeroize', '--test', 'dummy_fixed')

        all_features = '"alloc", "block-api", "mac", "oid", "zeroize", "dev"'
        (work / 'Cargo.toml').write_text(manifest(core).replace('default-features = false}',
            'default-features = false, features = [' + all_features + ']}'))
        cargo('test', '--locked', '--doc', '--package', 'cryptographic-core')

        # A disposable package copy tests compiler-enforced unsafe policy.
        copied = work / 'core-safety'
        shutil.copytree(core, copied)
        copied_manifest = copied / 'Cargo.toml'
        source = copied_manifest.read_text()
        source = re.sub(r'(?m)^path = "(\.\./[^\"]+)"$',
                        lambda match: 'path = "' + (core / match[1]).resolve().as_posix() + '"', source)
        # Crate documentation is also relative to its original owner.
        for file in (copied / 'src').rglob('*.rs'):
            value = file.read_text()
            value = re.sub(r'include_str!\("([^\"]+)"\)',
                           lambda match: 'include_str!("' + (core / file.relative_to(copied).parent / match[1]).resolve().as_posix() + '")', value)
            file.write_text(value)
        copied_manifest.write_text(source)
        (work / 'Cargo.toml').write_text(manifest(copied))
        cargo('generate-lockfile')
        for relative in ('src/lib.rs', 'src/hash/mod.rs', 'src/common_types/mod.rs'):
            file = copied / relative
            before = file.read_bytes()
            try:
                file.write_bytes(before + b'\nunsafe fn forbidden_safety_probe() {}\n')
                result = cargo('check', '--locked', '--no-default-features', capture=True)
                if not result.returncode or 'declaration of an `unsafe` function' not in result.stderr:
                    raise AssertionError(f'unsafe policy not enforced in {relative}: {result.stderr}')
            finally:
                file.write_bytes(before)
        (work / 'src/lib.rs').write_text(probe + 'pub use cryptographic_core::buffer::EagerBuffer;\n')
        try:
            result = cargo('check', '--locked', '--no-default-features', capture=True)
            if not result.returncode or 'unresolved import' not in result.stderr:
                raise AssertionError('buffer unexpectedly available without block-api')
        finally:
            (work / 'src/lib.rs').write_text(probe)
        cargo('check', '--locked', '--no-default-features')
    print('Cryptographic core: 16 minimal-feature and 2 all-feature checks, preserved buffer/zeroize tests and safety boundaries verified.')
