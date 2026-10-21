#!/usr/bin/env python3
"""Check the maintained TOML capability boundaries and real consumers offline."""
import argparse
import json
import re
import shutil
import subprocess
import tempfile
import sys
import tomllib
from pathlib import Path

FEATURE_SETS = ((), ('serialization',), ('write',), ('write', 'alloc'), ('edit',),
                ('parse',), ('display',), ('serialization', 'parse'),
                ('serialization', 'display'), ('preserve_order',),
                ('serialization', 'preserve_order'), ('parse', 'display'),
                ('serialization', 'parse', 'display', 'unbounded'),
                ('serialization', 'edit'),
                ('std', 'alloc', 'write', 'edit', 'parse', 'display',
                 'serialization', 'preserve_order', 'unbounded'))
REMOVED = ('toml_datetime', 'toml_edit', 'toml_write', 'source-spanned-value')
TARGETS = ('x86_64-unknown-linux-gnu', 'wasm32-unknown-unknown')


def command(operation, *args):
    return ['cargo', operation, '--locked', '--offline', *args]


def check_structure(root):
    for name in REMOVED:
        assert not (root / 'src/foundation' / name).exists(), f'old source root: {name}'
    manifest = tomllib.loads((root / 'src/foundation/toml/Cargo.toml').read_text())
    assert manifest['package']['name'] == 'toml'
    assert 'perf' not in manifest['features'], 'missing kstring-backed capability advertised'
    for features in FEATURE_SETS:
        assert set(features) <= set(manifest['features'])
    catalog = json.loads((root / 'compliance/rust-sources.json').read_text())
    origins = {Path(row['origin_directory']).name for row in
               [*catalog['packages'].values(), *catalog.get('merges', [])]
               if row['directory'] == 'src/foundation/toml'}
    assert origins == {'toml-0.8.23', 'toml_datetime-0.6.11', 'toml_edit-0.22.27',
                       'toml_write-0.1.2', 'serde_spanned-0.6.9'}, 'lost original source identity'


def fixture(root, directory, features):
    manifest = directory / 'Cargo.toml'
    manifest.write_text('[package]\nname="toml-capability-probe"\nversion="0.0.0"\nedition="2021"\n'
                        '[workspace]\n[dependencies]\ntoml={path='
                        + json.dumps(str(root / 'src/foundation/toml'))
                        + ',default-features=false,features=' + json.dumps(list(features)) + '}\n')
    source = directory / 'src/lib.rs'
    source.parent.mkdir(exist_ok=True)
    source.write_text('pub fn primitive(_: toml::datetime::Datetime, _: toml::span::Spanned<u8>) {}\n')
    subprocess.run(['cargo', 'generate-lockfile', '--offline', '--manifest-path', str(manifest)],
                   cwd=root, check=True)
    return manifest, source


def check_capabilities(root):
    with tempfile.TemporaryDirectory(prefix='toml-capabilities-') as temporary:
        directory = Path(temporary)
        for features in FEATURE_SETS:
            manifest, source = fixture(root, directory, features)
            for target in TARGETS:
                subprocess.run([sys.executable, str(root / '.github/scripts/check-rust-diagnostics.py'), '--',
                                *command('check', '--manifest-path', str(manifest), '--target', target, '--message-format=json')],
                               cwd=root, check=True)
            if not features:
                result = subprocess.run(command('metadata', '--manifest-path', str(manifest),
                                                '--format-version', '1', '--filter-platform', TARGETS[0]),
                                        cwd=root, capture_output=True, text=True, check=True)
                metadata = json.loads(result.stdout)
                names = {row['name'] for row in metadata['packages']}
                assert names == {'toml', 'toml-capability-probe'}, f'minimal dependency leak: {names}'
                for code, diagnostic in (
                    ('use toml::editor::DocumentMut;', 'editor'),
                    ('use toml::writer::TomlWrite;', 'writer'),
                    ('use toml::Value;', 'Value'),
                ):
                    source.write_text(code + '\n')
                    failure = subprocess.run(command('check', '--manifest-path', str(manifest)),
                                             cwd=root, capture_output=True, text=True)
                    assert failure.returncode != 0 and diagnostic in failure.stderr and (
                        'unresolved import' in failure.stderr or 'could not find' in failure.stderr
                    ), f'API isolation probe inconclusive: {failure.stderr}'
        manifest, _ = fixture(root, directory, ())
        failure = subprocess.run(command('check', '--manifest-path', str(manifest), '--features', 'toml/perf'),
                                 cwd=root, capture_output=True, text=True)
        assert failure.returncode != 0 and 'perf' in failure.stderr and 'does not have' in failure.stderr, (
            'removed capability probe inconclusive: ' + failure.stderr
        )


def check_retained_tests(root):
    # Foundation is excluded from the product workspace. Test a byte-exact
    # isolated source copy with its real dev dependencies, not a fake provider.
    with tempfile.TemporaryDirectory(prefix='toml-retained-tests-') as temporary:
        snapshot = Path(temporary)
        for relative in ('src/foundation/toml', 'test/foundation/toml',
                         'test/tools/toml', 'docs/components/toml'):
            shutil.copytree(root / relative, snapshot / relative)
        manifest = snapshot / 'src/foundation/toml/Cargo.toml'
        source_manifest = root / 'src/foundation/toml/Cargo.toml'
        text = manifest.read_text()
        # Only dependency locations are rewritten. Local source/test/doc paths
        # retain the same directory layout and exact original bytes.
        text = re.sub(r'(?m)^path = "(\.\./[^"\n]+)"$',
                      lambda match: 'path = ' + json.dumps(str((source_manifest.parent / match[1]).resolve()))
                      if not match[1].startswith('../../../test/') else match[0], text)
        manifest.write_text(text + '\n[workspace]\n')
        subprocess.run(['cargo', 'generate-lockfile', '--offline', '--manifest-path', str(manifest)],
                       cwd=root, check=True)
        subprocess.run(command('test', '--manifest-path', str(manifest), '--lib', '--test', 'capabilities'),
                       cwd=root, check=True)
        subprocess.run(command('test', '--manifest-path', str(manifest), '--doc'), cwd=root, check=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument('--structure-only', action='store_true')
    args = parser.parse_args()
    root = args.root.resolve()
    check_structure(root)
    if not args.structure_only:
        check_capabilities(root)
        check_retained_tests(root)
        subprocess.run(command('check', '-p', 'release-tools', '-p', 'contract-check'), cwd=root, check=True)
    print('TOML consolidation: PASS')


if __name__ == '__main__':
    main()
