#!/usr/bin/env python3
"""Run the entropy migration matrix without adding a repository workspace."""
from pathlib import Path
import json
import os
import re
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
CARGO = os.environ.get('CARGO', 'cargo')

def run(arguments, env=None):
    subprocess.run([CARGO, *arguments], cwd=ROOT, env=env, check=True)

def unit(directory, arguments):
    # Preserve CARGO_MANIFEST_DIR-based external includes without generating a
    # Cargo.lock inside the governed source package, even transiently.
    source = ROOT / 'src/foundation' / directory
    with tempfile.TemporaryDirectory(prefix='entropy-unit-') as temporary:
        isolated = Path(temporary)
        shutil.copyfile(ROOT / "rust-toolchain.toml", isolated / "rust-toolchain.toml")
        package = isolated / 'src/foundation' / directory
        shutil.copytree(source, package)
        external = ROOT / 'test/foundation' / directory
        if external.is_dir():
            shutil.copytree(external, isolated / 'test/foundation' / directory)
        for relative in ('test/entropy/error.rs', 'docs/components/system-entropy.md'):
            target = isolated / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / relative, target)
        manifest = package / 'Cargo.toml'
        text = manifest.read_text()
        text = re.sub(r'(path|readme) = "(\.\./[^"]+)"',
                      lambda match: match[1] + ' = ' + json.dumps(str((source / match[2]).resolve())),
                      text)
        manifest.write_text(text + '\n[workspace]\n')
        run(['generate-lockfile', '--offline', '--manifest-path', str(manifest)])
        run(['test', '--locked', '--offline', '--manifest-path', str(manifest), *arguments])

def main():
    for features in ([], ['--features', 'system-entropy'], ['--features', 'std']):
        unit('random-generator-core', ['--lib', '--no-default-features', *features])
    unit('system-entropy', ['--doc'])
    with tempfile.TemporaryDirectory(prefix='entropy-migration-') as temporary:
        shutil.copyfile(ROOT / 'rust-toolchain.toml', Path(temporary) / 'rust-toolchain.toml')
        manifest = Path(temporary) / 'Cargo.toml'
        source = lambda name: json.dumps(str(ROOT / 'src/foundation' / name))
        manifest.write_text(f'''[workspace]
[package]
name = "entropy-migration-probe"
version = "0.0.0"
edition = "2024"
[features]
std = ["random-generator-core/std", "system-entropy/std"]
browser = ["wasm_js"]
wasm_js = ["system-entropy/wasm_js"]
[dependencies]
random-generator-core = {{ path = {source('random-generator-core')}, features = ["system-entropy"] }}
system-entropy = {{ path = {source('system-entropy')} }}
[target.'cfg(target_os = "linux")'.dependencies]
ring = {{ path = {source('ring')} }}
[lib]
path = {json.dumps(str(ROOT / 'test/entropy/failure_probe.rs'))}
[[test]]
name = "system_entropy_source"
path = {json.dumps(str(ROOT / 'test/foundation/system-entropy/tests/mod.rs'))}
''')
        run(['generate-lockfile', '--offline', '--manifest-path', str(manifest)])
        run(['test', '--locked', '--offline', '--manifest-path', str(manifest), '--test', 'system_entropy_source'])
        env = os.environ.copy()
        if env.get('RUSTFLAGS') or env.get('CARGO_ENCODED_RUSTFLAGS'):
            raise SystemExit('Unset external Rust flags for a controlled entropy failure probe.')
        env['RUSTFLAGS'] = '--cfg system_entropy_backend="unsupported"'
        for features in ([], ['--features', 'std']):
            run(['test', '--locked', '--offline', '--manifest-path', str(manifest), '--lib', *features], env)
        run(['check', '--locked', '--offline', '--manifest-path', str(manifest), '--target', 'wasm32-unknown-unknown', '--features', 'browser'])
    print('Entropy matrix passed: core/no_std/std, docs, forced failures, browser WASM compile.')

if __name__ == '__main__':
    main()
