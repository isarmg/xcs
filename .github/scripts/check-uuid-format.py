#!/usr/bin/env python3
"""UUID feature consumers and differential/performance format validation.

Run: python3 .github/scripts/check-uuid-format.py [--bench]
The oracle is validation-only, never an active product dependency. No network,
root lockfile writes, or architecture-specific intrinsics are required.
"""
import os
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

from retired_sources import restore

ROOT = Path(__file__).resolve().parents[2]
RETIRED_FEATURES = ('atomic', 'borsh', 'macro-diagnostics', 'md5',
                    'rng-system-entropy', 'sha1', 'v1', 'v3', 'v5', 'v6')
CONSUMER_TESTS = {'v4_generation_preserves_version_variant_and_roundtrips'}


def check_features(root=ROOT):
    """Exercise the live UUID component, separately from the format oracle."""
    with tempfile.TemporaryDirectory(prefix='uuid-features-') as temporary:
        work = Path(temporary)
        manifest = work / 'Cargo.toml'
        manifest.write_text(
            '[package]\nname = "uuid-feature-contracts"\nversion = "0.0.0"\n'
            'edition = "2021"\n[workspace]\n[dependencies.uuid]\n'
            f'path = {json.dumps(str(root / "src/foundation/core-types/uuid"))}\n'
            'features = ["v4", "js"]\n[[test]]\nname = "contracts"\n'
            f'path = {json.dumps(str(root / "test/ci/fixtures/uuid-features.rs"))}\n')
        (work / 'rust-toolchain.toml').write_bytes((root / 'rust-toolchain.toml').read_bytes())
        env = dict(os.environ, CARGO_BUILD_JOBS='1', CARGO_INCREMENTAL='0')
        env.setdefault('CARGO_TARGET_DIR', str(root / 'target/native'))

        def run(*args, check=True):
            cargo_args, test_args = list(args), []
            if '--' in args:
                separator = args.index('--')
                cargo_args, test_args = list(args[:separator]), ['--', *args[separator + 1:]]
            command = ['cargo', *cargo_args, '--offline', '--color', 'never',
                       '--manifest-path', str(manifest), *test_args]
            try:
                result = subprocess.run(command, cwd=root, env=env, capture_output=True,
                                        text=True, check=check)
            except subprocess.CalledProcessError as error:
                print(error.stdout or '', end='')
                print(error.stderr or '', end='', file=sys.stderr)
                raise
            print(result.stdout, end='')
            print(result.stderr, end='', file=sys.stderr)
            return result

        run('generate-lockfile')
        listing = run('test', '--locked', '--test', 'contracts', '--', '--list')
        discovered = {line.removesuffix(': test') for line in listing.stdout.splitlines()
                      if line.endswith(': test')}
        if discovered != CONSUMER_TESTS:
            raise AssertionError(f'UUID consumer tests not discovered: {discovered}')
        run('test', '--locked', '--test', 'contracts')
        run('check', '--locked', '--tests', '--target', 'wasm32-unknown-unknown')
        run('check', '--locked', '--tests', '--features', 'uuid/v7,uuid/v8,uuid/serialization')
        for feature in RETIRED_FEATURES:
            result = run('check', '--locked', '--tests', '--features', f'uuid/{feature}', check=False)
            if (result.returncode == 0 or 'does not have that feature' not in result.stderr
                    or f'feature `{feature}`' not in result.stderr or '`uuid`' not in result.stderr):
                raise AssertionError(f'UUID feature {feature} was accepted or failed for another reason: '
                                     + result.stderr)
    print('PASS: UUID native v4+js consumers, browser WASM compilation, and retired feature rejection')
    print('WASM coverage is a compile gate, not browser runtime entropy execution.')

def main():
    check_features()
    with tempfile.TemporaryDirectory(prefix='uuid-format-') as temp:
        work = Path(temp)
        paths = ['src/foundation/' + name for name in ('uuid-simd', 'vsimd', 'outref')]
        restore(ROOT, work, ('uuid-simd', 'vsimd', 'outref'))
        # The disabled optional uuid feature is not part of the oracle contract.
        manifest = work / paths[0] / 'Cargo.toml'
        text = manifest.read_text()
        start = text.index('[dependencies.uuid]')
        end = text.index('[dependencies.vsimd]', start)
        manifest.write_text(text[:start] + text[end:].replace('"detect", "uuid"', '"detect"') + '\nuuid = []\n')
        (work / 'Cargo.toml').write_text('''[package]
name = "uuid-format-probe"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
uuid-simd = { path = "src/foundation/uuid-simd", default-features = false, features = ["std", "detect"] }
''')
        shutil.copy(ROOT / 'test/schema-validation/uuid_format_probe.rs', work / 'src/main.rs')
        shutil.copy(ROOT / 'src/foundation/data-storage/schema-validation/src/validation/keywords/uuid_format.rs', work / 'src/production.rs')
        subprocess.run(['rustc', '--edition=2021', '--crate-type=lib', '--target', 'wasm32-unknown-unknown',
                        str(work / 'src/production.rs'), '-o', str(work / 'uuid-format.wasm.rlib')], check=True)
        env = dict(os.environ, CARGO_BUILD_JOBS='1', CARGO_INCREMENTAL='0', CARGO_PROFILE_RELEASE_DEBUG='0')
        env['CARGO_TARGET_DIR'] = str(work / 'target')
        subprocess.run(['cargo', 'run', '--release', '--offline', '--manifest-path', str(work / 'Cargo.toml'), '--', *sys.argv[1:]], env=env, check=True)


if __name__ == '__main__':
    main()
