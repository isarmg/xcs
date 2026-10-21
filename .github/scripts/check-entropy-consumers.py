#!/usr/bin/env python3
"""Check maintained default hash-seeding failure behavior against the live entropy implementation."""
import argparse
from pathlib import Path
import runpy
import tempfile

ROOT = Path(__file__).resolve().parents[2]

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--target-dir', type=Path, default=ROOT / 'target/entropy-consumers')
    args = parser.parse_args()
    backend = runpy.run_path(str(ROOT / '.github/scripts/check-linux-random-backends.py'))
    env = backend['matrix_environment']()
    env['RUSTFLAGS'] = '--deny=warnings --cfg system_entropy_backend="unsupported"'
    with tempfile.TemporaryDirectory(prefix='entropy-consumers-') as directory:
        (Path(directory) / 'rust-toolchain.toml').write_bytes((ROOT / 'rust-toolchain.toml').read_bytes())
        manifest = Path(directory) / 'Cargo.toml'
        # Path identities must track the separately reviewed role-naming change.
        manifest.write_text('[package]\nname="entropy-consumer-regression"\nversion="0.0.0"\nedition="2024"\n'
                            '[workspace]\n[features]\nbrowser=["system-entropy/wasm_js"]\n[dependencies]\n'
                            f'ahash={{path="{ROOT / "src/foundation/ahash"}"}}\n'
                            f'system-entropy={{path="{ROOT / "src/foundation/system-entropy"}"}}\n'
                            '[[test]]\nname="isolated_consumer"\n'
                            f'path="{ROOT / "test/ci/fixtures/entropy-fail-closed.rs"}"\n')
        backend['run'](['cargo', 'generate-lockfile', '--offline', '--manifest-path', str(manifest)], env)
        backend['run'](['cargo', 'test', '--no-run', '--locked', '--offline', '-j2',
                        '--manifest-path', str(manifest), '--target-dir', str(args.target_dir.resolve()),
                        '--test', 'isolated_consumer', '--message-format=json'], env)
        browser_env = backend['matrix_environment']()
        browser_env['RUSTFLAGS'] = '--deny=warnings'
        common = ['--locked', '--offline', '-j2', '--manifest-path', str(manifest),
                  '--target-dir', str(args.target_dir.resolve()), '--test', 'isolated_consumer',
                  '--target', 'wasm32-unknown-unknown', '--message-format=json']
        backend['run'](['cargo', 'check', *common], browser_env,
                       'are not supported by default')
        backend['run'](['cargo', 'check', *common, '--features', 'browser'], browser_env)
    print('PASS: two native fail-closed API/seeding tests; WASM rejects missing wasm_js and compiles with it')
    print('WASM checks are compile gates, not a browser runtime entropy assertion.')

if __name__ == '__main__':
    main()
