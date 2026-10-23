#!/usr/bin/env python3
"""Check real feature-isolated compression sources, vectors, streams and consumers."""
from pathlib import Path
import json
import os
import re
import shutil
import subprocess
import tempfile
import tomllib

import runpy

ROOT = Path(__file__).resolve().parents[2]
foundation_path = runpy.run_path(str(ROOT / '.github/scripts/foundation_paths.py'))['package_path']
FEATURES = ('std', 'alloc', 'codec', 'streams', 'simd', 'runtime-detection', 'block-boundary', 'serialization')
REMOVED = ('any_c_zlib', 'any_impl', 'any_zlib', 'cloudflare_zlib', 'miniz-sys', 'miniz_oxide', 'rust_backend', 'zlib', 'zlib-default', 'zlib-ng', 'zlib-ng-compat', 'nightly', 'rustc-dep-of-std', 'zlib-rs', 'const-generics', 'with-alloc', 'runtime_detection')
MATRIX = ('', 'std', 'alloc', 'codec', 'codec,alloc', 'simd', 'simd,std', 'codec,simd', 'streams', 'streams,simd,runtime-detection', 'block-boundary', 'serialization', ','.join(FEATURES))


def check(root=ROOT):
    source = root / 'src/foundation/data-storage/compression'
    manifest = tomllib.loads((source / 'Cargo.toml').read_text())
    assert set(manifest['features']) == {'default', *FEATURES}
    assert manifest['features']['default'] == []
    for old in ('flate2', 'miniz_oxide', 'adler2', 'simd-adler32'):
        assert not (foundation_path(root, old)).exists(), old
    # No copied/fake dependency providers. Only this package and its external tests
    # are mirrored so Cargo cannot create locks or doctest files in governed trees.
    with tempfile.TemporaryDirectory(prefix='zxs-compression-') as folder:
        work = Path(folder)
        shutil.copyfile(root / 'rust-toolchain.toml', work / 'rust-toolchain.toml')
        mirror = work / 'src/foundation/data-storage/compression'
        shutil.copytree(source, mirror)
        for relative in ('test/compression', 'test/foundation/compression'):
            shutil.copytree(root / relative, work / relative)
        text = (mirror / 'Cargo.toml').read_text()
        for dependency, row in manifest['dependencies'].items():
            text = text.replace(f'path = "{row["path"]}"', f'path = "{(source / row["path"]).resolve()}"')
        (mirror / 'Cargo.toml').write_text(text)
        env = dict(os.environ, CARGO_BUILD_JOBS='1', CARGO_INCREMENTAL='0', CARGO_PROFILE_DEV_DEBUG='0', CARGO_PROFILE_TEST_DEBUG='0', CARGO_TARGET_DIR=str(work / 'target'))
        def run(command, *args, success=True):
            result = subprocess.run(['cargo', command, '--color', 'never', '--manifest-path', str(mirror / 'Cargo.toml'), '--offline', *args], cwd=work, env=env, text=True, capture_output=True)
            if success and (result.returncode or re.search(r'^warning:', result.stderr, re.M)):
                raise AssertionError(result.stdout + result.stderr)
            return result
        run('generate-lockfile')
        for features, expected in (('', {'compression'}), ('codec', {'compression'}), ('codec,alloc', {'compression'}), ('simd', {'compression'}), ('streams', {'compression', 'crc32fast', 'cfg-if'})):
            flags = ['--features', features] if features else []
            metadata = json.loads(run('metadata', '--locked', '--format-version', '1', *flags).stdout)
            assert {p['name'] for p in metadata['packages']} == expected, (features, metadata['packages'])
        for target in ('x86_64-unknown-linux-gnu', 'wasm32-unknown-unknown'):
            for features in MATRIX:
                flags = ['--features', features] if features else []
                run('check', '--locked', '--target', target, *flags)
                print(f'compression matrix PASS: {target}: {features or "checksum-only"}', flush=True)
        for features in ('', 'simd', 'codec,alloc', 'streams', ','.join(FEATURES)):
            flags = ['--features', features] if features else []
            result = run('test', '--locked', '--tests', *flags)
            print(result.stdout, end='', flush=True)
        print(run('test', '--locked', '--doc', '--all-features').stdout, end='', flush=True)
        # Check all removed flags reject real Cargo selection, not just manifest text.
        for feature in REMOVED:
            result = run('check', '--locked', '--features', feature, success=False)
            assert result.returncode and 'does not contain this feature' in result.stderr, (feature, result.stderr)
        probe = work / 'probe'
        (probe / 'src').mkdir(parents=True)
        (probe / 'Cargo.toml').write_text(f'[package]\nname = "compression-api-probe"\nversion = "0.0.0"\nedition = "2021"\n[workspace]\n[dependencies.compression]\npath = "{mirror}"\ndefault-features = false\n')
        (probe / 'src/lib.rs').write_text('')
        subprocess.run(['cargo', 'generate-lockfile', '--color', 'never', '--offline', '--manifest-path', str(probe / 'Cargo.toml')], cwd=work, env=env, check=True)
        for name in ('codec', 'streams', 'accelerated_checksum'):
            (probe / 'src/lib.rs').write_text(f'pub use compression::{name};\n')
            result = subprocess.run(['cargo', 'check', '--locked', '--color', 'never', '--offline', '--manifest-path', str(probe / 'Cargo.toml')], cwd=work, env=env, text=True, capture_output=True)
            assert result.returncode and 'error[E0432]' in result.stderr and f'compression::{name}' in result.stderr, result.stderr
        # A real serialization consumer checkpoints and resumes codec state.
        (probe / 'src/lib.rs').write_text('')
        (probe / 'Cargo.toml').write_text(f'''[package]
name = "compression-checkpoint-probe"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies.compression]
path = "{mirror}"
default-features = false
features = ["serialization"]
[dependencies.json-codec]
path = "{root / 'src/foundation/data-storage/json-codec'}"
[[test]]
name = "checkpoint"
path = "{root / 'test/compression/serialization.rs'}"
''')
        for args in (['generate-lockfile'], ['test', '--locked']):
            result = subprocess.run(['cargo', *args, '--color', 'never', '--offline', '--manifest-path', str(probe / 'Cargo.toml')], cwd=work, env=env, text=True, capture_output=True)
            assert result.returncode == 0 and not re.search(r'^warning:', result.stderr, re.M), result.stdout + result.stderr
            print(result.stdout, end='', flush=True)
    # Exercise the actual builder that creates release gzip assets.
    subprocess.run(['cargo', 'test', '--locked', '--offline', '-p', 'web-build'], cwd=root, check=True)
    print('Compression: lightweight checksum/codec closure, 26 native/WASM checks, real tests/doctests, disabled API and backend rejection, actual web-build consumer verified.')


if __name__ == '__main__':
    check()
