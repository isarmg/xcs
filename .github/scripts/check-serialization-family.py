#!/usr/bin/env python3
"""Check serialization protocol bytes and coordinated derives with isolated consumers."""
from pathlib import Path
import json
import re
import runpy
import subprocess
import sys
import tempfile
import tomllib

ROOT = Path(__file__).resolve().parents[2]
NATIVE = "x86_64-unknown-linux-gnu"
WASM = "wasm32-unknown-unknown"



def validate_private_family(manifests):
    names = ('data-serialization', 'serialization-derive')
    version = manifests[names[0]]['package']['version']
    for name in names:
        if manifests[name]['package']['version'] != version:
            raise AssertionError('serialization private family package versions must match')
    dependencies = manifests[names[0]]['dependencies']
    for name in names[1:]:
        if dependencies[name].get('version') != '=' + version:
            raise AssertionError(f'{name} must be pinned to ={version} for the shared private ABI')


def check_source_paths(root):
    # Also scan strings: derive bounds are Rust paths embedded in attributes.
    # Underscored external APIs and macro-local aliases are different identities.
    stale = re.compile(r'(?<![A-Za-z0-9_])serde::')
    for directory in ('src', 'tools'):
        for path in (root / directory).rglob('*.rs'):
            if stale.search(path.read_text()):
                raise AssertionError(f'old serialization crate path: {path.relative_to(root)}')


def check_private_family(root):
    manifests = {name: tomllib.loads((root / 'src/foundation' / name / 'Cargo.toml').read_text())
                 for name in ('data-serialization', 'serialization-derive')}
    validate_private_family(manifests)


def manifest(root, no_std=False):
    foundation = root / "src/foundation"
    dependencies = {
        "data-serialization": (['derive', 'alloc', 'rc', 'error-path'], not no_std),
        "json-codec": (['alloc'] if no_std else ['raw_value', 'arbitrary_precision'], not no_std),
    }
    if not no_std:
        for name in ('flag-set', 'miniz_oxide', 'icu_collections', 'random-generator-core',
                     'derive-where'):
            dependencies[name] = (['serialization'], True)
        dependencies['toml'] = (['serialization'], False)
        dependencies['time'] = (['serialization', 'serialization-human-readable'], True)
        dependencies['form-codec'] = ([], True)
    lines = ['[package]', 'name = "serialization-family-regression"', 'version = "0.0.0"',
             'edition = "2021"', '[workspace]', '[dependencies]']
    for name, (features, defaults) in dependencies.items():
        lines.append(f'{name} = {{ path = {json.dumps(str(foundation / name))}, '
                     f'features = {json.dumps(features)}, default-features = {str(defaults).lower()} }}')
    return '\n'.join(lines) + '\n'


def build(root, work, environment, target, tests=False):
    diagnostics_type = runpy.run_path(str(root / '.github/scripts/check-rust-diagnostics.py'))['CargoDiagnostics']
    command = ['cargo', 'test' if tests else 'check', '--locked', '--offline',
               '--manifest-path', str(work / 'Cargo.toml'), '--target', target,
               '--target-dir', str(work.parent / 'target'), '--message-format=json', '-j2']
    if tests:
        command.append('--no-run')
    diagnostics = diagnostics_type(sys.stderr)
    executables = []
    with subprocess.Popen(command, cwd=root, env=environment, stdout=subprocess.PIPE,
                          text=True, encoding='utf-8') as process:
        for number, line in enumerate(process.stdout, 1):
            diagnostics.consume(line, number)
            try:
                event = json.loads(line)
            except ValueError:
                continue
            if (isinstance(event, dict) and event.get('reason') == 'compiler-artifact'
                    and event.get('target', {}).get('name') == 'serialization_family_regression'
                    and event.get('profile', {}).get('test') and event.get('executable')):
                executables.append(event['executable'])
        if diagnostics.result(process.wait()):
            raise RuntimeError('serialization consumer diagnostics rejected')
    if tests:
        if len(executables) != 1:
            raise AssertionError('expected exactly one serialization test executable')
        subprocess.run([executables[0]], cwd=root, env=environment, check=True)



def check_rejections(root, directory, environment):
    work = directory / 'rejected'
    (work / 'src').mkdir(parents=True)
    (work / 'Cargo.toml').write_text(manifest(root, no_std=True))
    (work / 'src/lib.rs').write_text('')
    subprocess.run(['cargo', 'generate-lockfile', '--offline', '--manifest-path',
                    str(work / 'Cargo.toml')], cwd=root, env=environment, check=True)
    cases = [
        ('#[derive(data_serialization::Serialize)]\n#[serde(rename_all="camelCase")]\n'
         'pub struct Rejected { field_name: u32 }\n', 'cannot find attribute `serde`'),
        ('#[derive(data_serialization::Serialize)]\n#[serialization(crate="serialization_core")]\n'
         'pub struct Rejected { value: u32 }\n',
         'serialization_core'),
    ]
    for source, expected in cases:
        (work / 'src/lib.rs').write_text(source)
        result = subprocess.run(['cargo', 'check', '--locked', '--offline', '--manifest-path',
                                 str(work / 'Cargo.toml'), '--target', NATIVE,
                                 '--target-dir', str(directory / 'target'), '-j2'],
                                cwd=root, env=environment, capture_output=True, text=True)
        if result.returncode == 0 or expected not in result.stderr:
            raise AssertionError(f'expected rejection {expected!r}: {result.stdout} {result.stderr}')
        print(f'Confirmed rejection: {expected}', flush=True)


def check(root):
    check_source_paths(root)
    check_private_family(root)
    environment = runpy.run_path(str(root / '.github/scripts/backend_environment.py'))['matrix_environment']()
    (root / 'target').mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='serialization-regression-', dir=root / 'target') as temporary:
        for no_std in (False, True):
            work = Path(temporary) / ('no-std' if no_std else 'standard')
            (work / 'src').mkdir(parents=True)
            (work / 'Cargo.toml').write_text(manifest(root, no_std))
            fixture = 'serialization-no-std.rs' if no_std else 'serialization-family.rs'
            (work / 'src/lib.rs').write_text((root / 'test/ci/fixtures' / fixture).read_text())
            subprocess.run(['cargo', 'generate-lockfile', '--offline', '--manifest-path',
                            str(work / 'Cargo.toml')], cwd=root, env=environment, check=True)
            build(root, work, environment, NATIVE, tests=not no_std)
            build(root, work, environment, WASM)
            # The absorbed span doctests now run in check-toml-consolidation.py
            # with the real dev dependencies, alongside all other TOML docs.
        check_rejections(root, Path(temporary), environment)


if __name__ == '__main__':
    check(ROOT)
