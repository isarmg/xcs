#!/usr/bin/env python3
"""Verify the project-owned tracing facade, collectors, and typed JSON output."""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib

FEATURES = ((), ('std',), ('collector',), ('registry',), ('fmt',), ('env-filter',),
            ('pattern',), ('pattern', 'unicode'), ('log',), ('log-tracer',),
            ('json',), ('collector-default', 'env-filter', 'json', 'time', 'attributes', 'log'))
TARGETS = ('x86_64-unknown-linux-gnu', 'wasm32-unknown-unknown')
ABSORBED = ('tracing-core', 'tracing-subscriber', 'tracing-log', 'matchers')
FULL = FEATURES[-1]


def run(root, args):
    subprocess.run(args, cwd=root, check=True)


def structure(root):
    package = tomllib.loads((root / 'src/foundation/tracing/Cargo.toml').read_text())
    assert package['package']['name'] == 'tracing'
    assert not {'interest-cache', 'local-time', 'valuable', 'async-await'} & package['features'].keys()
    assert all(not (root / 'src/foundation' / name).exists() for name in ABSORBED)
    assert not set(ABSORBED) & package['dependencies'].keys()
    assert not {'tracing-serde', 'lru'} & package['dependencies'].keys()
    assert package['dependencies']['log']['optional']
    assert 'parking_lot' not in package['features']
    assert package['dependencies']['synchronization']['default-features'] is False
    assert 'synchronization/locks' in package['features']['parking']


def capabilities(root):
    with tempfile.TemporaryDirectory(prefix='tracing-capabilities-') as temp:
        d = Path(temp); (d / 'src').mkdir()
        shutil.copyfile(root / 'rust-toolchain.toml', d / 'rust-toolchain.toml')
        manifest = d / 'Cargo.toml'; source = d / 'src/lib.rs'
        for features in FEATURES:
            manifest.write_text('[package]\nname="tracing-probe"\nversion="0.0.0"\nedition="2021"\n[workspace]\n[dependencies]\ntracing={path=' + json.dumps(str(root / 'src/foundation/tracing')) + ',default-features=false,features=' + json.dumps(features) + '}\n')
            source.write_text('#![no_std]\npub fn instrument() { tracing::info!(answer = 42u64); }\n')
            run(root, ['cargo', 'generate-lockfile', '--offline', '--manifest-path', str(manifest)])
            for target in TARGETS:
                run(root, [sys.executable, str(root / '.github/scripts/check-rust-diagnostics.py'), '--', 'cargo', 'check', '--locked', '--offline', '--manifest-path', str(manifest), '--target', target, '--message-format=json'])
            if not features:
                result = subprocess.run(['cargo', 'metadata', '--locked', '--offline', '--manifest-path', str(manifest), '--format-version', '1'], cwd=root, capture_output=True, text=True, check=True)
                names = {p['name'] for p in json.loads(result.stdout)['packages']}
                assert names == {'tracing-probe', 'tracing', 'pin-project-lite'}, names
                for item in ('collector', 'log_bridge', 'pattern'):
                    source.write_text('use tracing::' + item + ';\n')
                    negative = subprocess.run(['cargo', 'check', '--locked', '--offline', '--manifest-path', str(manifest)], cwd=root, capture_output=True, text=True)
                    assert negative.returncode and item in negative.stderr and 'unresolved import' in negative.stderr, negative.stderr
                source.write_text('#![no_std]\n')
                for removed in ('valuable', 'local-time', 'interest-cache', 'async-await'):
                    negative = subprocess.run(['cargo', 'check', '--locked', '--offline', '--manifest-path', str(manifest), '--features', 'tracing/' + removed], cwd=root, capture_output=True, text=True)
                    assert negative.returncode and removed in negative.stderr and 'does not have' in negative.stderr, negative.stderr


def retained_tests(root):
    with tempfile.TemporaryDirectory(prefix='tracing-retained-tests-') as temp:
        snapshot = Path(temp)
        shutil.copyfile(root / 'rust-toolchain.toml', snapshot / 'rust-toolchain.toml')
        for relative in ('src/foundation/tracing', 'test/foundation/tracing-family', 'test/tracing'):
            shutil.copytree(root / relative, snapshot / relative, ignore=shutil.ignore_patterns('Cargo.lock', 'target'))
        manifest = snapshot / 'src/foundation/tracing/Cargo.toml'
        text = manifest.read_text()
        text = re.sub(r'(?m)^path = "(\.\./[^"\n]+)"$', lambda m: 'path = ' + json.dumps(str((root / 'src/foundation/tracing' / m[1]).resolve())) if not m[1].startswith('../../../test/') else m[0], text)
        for p in sorted((snapshot / 'test/foundation/tracing-family/primitives/tests').glob('*.rs')):
            text += '\n[[test]]\nname=' + json.dumps('primitives-' + p.stem) + '\npath=' + json.dumps(str(p)) + '\n'
        for name in ('log_tracer', 'reexport_log_crate'):
            p = snapshot / 'test/foundation/tracing-family/log_bridge/tests' / (name + '.rs')
            text += '\n[[test]]\nname=' + json.dumps('bridge-' + name) + '\npath=' + json.dumps(str(p)) + '\n'
        manifest.write_text(text + '\n[workspace]\n')
        run(root, ['cargo', 'generate-lockfile', '--offline', '--manifest-path', str(manifest)])
        run(root, ['cargo', 'test', '--locked', '--offline', '--manifest-path', str(manifest), '--lib', '--tests', '--features', ','.join(FULL)])


def static_levels(root):
    with tempfile.TemporaryDirectory(prefix='tracing-static-levels-') as temp:
        d = Path(temp)
        shutil.copyfile(root / 'rust-toolchain.toml', d / 'rust-toolchain.toml')
        manifest = d / 'Cargo.toml'
        manifest.write_text('[package]\nname="tracing-static-levels"\nversion="0.0.0"\nedition="2021"\n[workspace]\n[dependencies]\ntracing={path=' + json.dumps(str(root / 'src/foundation/tracing')) + ',default-features=false,features=["registry","max_level_warn","release_max_level_error"]}\n[[test]]\nname="static-levels"\npath=' + json.dumps(str(root / 'test/tracing/static_levels.rs')) + '\n')
        run(root, ['cargo', 'generate-lockfile', '--offline', '--manifest-path', str(manifest)])
        for profile in ([], ['--release']):
            run(root, ['cargo', 'test', '--locked', '--offline', '--manifest-path', str(manifest), *profile])


def parking_registry(root):
    with tempfile.TemporaryDirectory(prefix='tracing-parking-') as temp:
        shutil.copyfile(root / 'rust-toolchain.toml', Path(temp) / 'rust-toolchain.toml')
        manifest = Path(temp) / 'Cargo.toml'
        manifest.write_text('[package]\nname="tracing-parking"\nversion="0.0.0"\nedition="2021"\n[workspace]\n[dependencies]\ntracing={path=' + json.dumps(str(root / 'src/foundation/tracing')) + ',default-features=false,features=["registry","parking"]}\n[[test]]\nname="parking"\npath=' + json.dumps(str(root / 'test/tracing/parking.rs')) + '\n')
        run(root, ['cargo', 'generate-lockfile', '--offline', '--manifest-path', str(manifest)])
        run(root, [sys.executable, str(root / '.github/scripts/check-rust-diagnostics.py'), '--', 'cargo', 'check', '--locked', '--offline', '--manifest-path', str(manifest), '--tests', '--message-format=json'])
        run(root, [sys.executable, str(root / '.github/scripts/check-rust-diagnostics.py'), '--', 'cargo', 'check', '--locked', '--offline', '--manifest-path', str(manifest), '--tests', '--target', 'wasm32-unknown-unknown', '--message-format=json'])
        run(root, ['cargo', 'test', '--locked', '--offline', '--manifest-path', str(manifest)])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument('--structure-only', action='store_true')
    args = parser.parse_args(); root = args.root.resolve()
    structure(root)
    if not args.structure_only:
        capabilities(root)
        retained_tests(root)
        static_levels(root)
        parking_registry(root)
    print('tracing family: PASS')


if __name__ == '__main__':
    main()
