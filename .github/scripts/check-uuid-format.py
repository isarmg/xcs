#!/usr/bin/env python3
"""Differential/performance UUID gate, with a catalog-authenticated retired oracle.

Run: python3 .github/scripts/check-uuid-format.py [--bench]
The oracle is validation-only, never an active product dependency. No network,
root lockfile writes, or architecture-specific intrinsics are required.
"""
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

from retired_sources import restore

ROOT = Path(__file__).resolve().parents[2]

def main():
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
