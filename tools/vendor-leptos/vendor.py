#!/usr/bin/env python3
"""Import pinned Leptos archives, or verify the maintained tree (Python 3.11+)."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tarfile
import tomllib

ROOT = Path(__file__).resolve().parents[2]
DEST = ROOT / 'vendor/web/dependencies'
PROVENANCE = DEST / 'provenance.json'
REPO = 'https://github.com/leptos-rs/leptos'


def sha(data):
    return hashlib.sha256(data).hexdigest()


def wire(text, packages):
    for name, package in packages.items():
        # Normalized registry manifests use dependency tables, including target/dev edges.
        pattern = rf'(?m)^(\[(?:[^\n]*\.)?(?:dependencies|dev-dependencies|build-dependencies)\.{re.escape(name)}\])$'
        text = re.sub(pattern, lambda m: m[1] + f'\npath = "../{name}-{package["version"]}"', text)
    return text + '\n# Maintained independently of the enclosing application workspace.\n[workspace]\n'


def import_archives(seed=None):
    if DEST.exists():
        raise SystemExit('Refusing to overwrite maintained sources; import into a clean worktree.')
    cache = Path(os.environ.get('CARGO_HOME', Path.home() / '.cargo')) / 'registry/cache'
    lock = tomllib.loads((ROOT / 'web/Cargo.lock').read_text())
    recorded = seed or ROOT / 'vendor/web/dependencies/provenance.json'
    if recorded.exists():
        # Reproduce the original import even after consumer lockfiles use paths.
        old = json.loads(recorded.read_text())['packages']
        for p in lock['package']:
            if p['name'] in old:
                p['source'] = old[p['name']]['registry_source']
                p['checksum'] = old[p['name']]['archive_sha256']
    packages = {}
    archives = {}
    for p in lock['package']:
        if not p.get('source', '').startswith('registry+'):
            continue
        candidates = list(cache.glob(f'*/{p["name"]}-{p["version"]}.crate'))
        if not candidates:
            # A lockfile also contains target-only ecosystem crates not cached here.
            # The selected Leptos family is explicitly audited below.
            continue
        archive = candidates[0]
        with tarfile.open(archive) as tar:
            prefix = f'{p["name"]}-{p["version"]}/'
            manifest = tomllib.loads(tar.extractfile(prefix + 'Cargo.toml').read().decode())
            if manifest['package'].get('repository', '').rstrip('/') != REPO:
                continue
            if sha(archive.read_bytes()) != p['checksum']:
                raise SystemExit(f'Archive checksum mismatch: {archive}')
        packages[p['name']] = dict(version=p['version'], registry_source=p['source'], archive_sha256=p['checksum'], archive_url=f'https://static.crates.io/crates/{p["name"]}/{p["name"]}-{p["version"]}.crate', repository=REPO, license=manifest['package']['license'])
        archives[p['name']] = archive
    required = {'any_spawner', 'const_str_slice_concat', 'either_of', 'leptos_dom', 'leptos_hot_reload', 'next_tuple', 'oco_ref', 'or_poisoned', 'reactive_graph', 'tachys', 'throw_error'}
    if set(packages) != required:
        raise SystemExit(f'Family scope changed or cache incomplete: expected {sorted(required)}, found {sorted(packages)}')
    DEST.mkdir(parents=True)
    for name, p in sorted(packages.items()):
        directory = DEST / f'{name}-{p["version"]}'
        with tarfile.open(archives[name]) as tar:
            tar.extractall(DEST, filter='data')
        p['upstream_files'] = {str(f.relative_to(directory)): sha(f.read_bytes()) for f in sorted(directory.rglob('*')) if f.is_file()}
        p['vcs'] = json.loads((directory / '.cargo_vcs_info.json').read_text())
        manifest = directory / 'Cargo.toml'
        manifest.write_text(wire(manifest.read_text(), packages))
        p['maintained_manifest_sha256'] = sha(manifest.read_bytes())
    PROVENANCE.write_text(json.dumps(dict(schema=1, packages=packages), indent=2, sort_keys=True) + '\n')
    print(f'Imported {len(packages)} verified archives.')


def check_resolution(entries, packages, label, metadata=False):
    errors = []
    for name, package in packages.items():
        selected = [p for p in entries if p['name'] == name]
        if len(selected) != 1:
            errors.append(f'{label}: expected exactly one {name}, found {len(selected)}')
        for p in selected:
            if p.get('source') or p['version'] != package['version']:
                errors.append(f'{label}: nonlocal/wrong-version package {name}')
            if metadata and Path(p['manifest_path']).resolve() != (DEST / f'{name}-{package["version"]}' / 'Cargo.toml').resolve():
                errors.append(f'{label}: unexpected manifest path for {name}')
    return errors


def patched_files(upstream, patches):
    expected = dict(upstream)
    for file, patch in patches.items():
        path = Path(file)
        if path.is_absolute() or '..' in path.parts or not patch.get('reason', '').strip():
            raise ValueError(f'Invalid patch record: {file}')
        digest = patch.get('sha256')
        if digest is None:
            if file not in expected:
                raise ValueError(f'Deletion of unknown upstream file: {file}')
            expected.pop(file)
        elif re.fullmatch('[0-9a-f]{64}', digest):
            expected[file] = digest
        else:
            raise ValueError(f'Invalid patch hash: {file}')
    return expected


def verify(check_metadata=False):
    packages = json.loads(PROVENANCE.read_text())['packages']
    errors = []
    patches = json.loads((DEST / 'patches.json').read_text())['packages']
    if set(patches) - set(packages):
        errors.append('Patch records contain unknown packages')
    for name, p in packages.items():
        directory = DEST / f'{name}-{p["version"]}'
        expected = patched_files(dict(p['upstream_files'], **{'Cargo.toml': p['maintained_manifest_sha256']}), patches.get(name, {}))
        for file in directory.rglob('*'):
            if file.is_symlink():
                errors.append(f'Symlink not allowed: {file.relative_to(ROOT)}')
        actual = {str(f.relative_to(directory)) for f in directory.rglob('*') if f.is_file() and 'target' not in f.relative_to(directory).parts}
        for extra in actual - set(expected):
            errors.append(f'Unrecorded file: {(directory / extra).relative_to(ROOT)}')
        for file, digest in expected.items():
            path = directory / file
            if not path.is_file() or sha(path.read_bytes()) != digest:
                errors.append(f'Unrecorded change: {path.relative_to(ROOT)}')
        manifest = tomllib.loads((directory / 'Cargo.toml').read_text())
        def check_edges(table):
            for key, value in table.items():
                if key in ('dependencies', 'dev-dependencies', 'build-dependencies'):
                    for dep, spec in value.items():
                        if dep in packages and (not isinstance(spec, dict) or spec.get('path') != f'../{dep}-{packages[dep]["version"]}'):
                            errors.append(f'Nonlocal dependency edge: {name} -> {dep}')
                elif isinstance(value, dict):
                    check_edges(value)
        check_edges(manifest)
        if (manifest['package']['name'], manifest['package']['version']) != (name, p['version']):
            errors.append(f'Identity mismatch: {name}')
    for consumer in ['web', 'demos/branch-system']:
        lock = tomllib.loads((ROOT / consumer / 'Cargo.lock').read_text())
        errors.extend(check_resolution(lock['package'], packages, consumer))
        if check_metadata:
            result = subprocess.run(['cargo', 'metadata', '--offline', '--locked', '--manifest-path', str(ROOT / consumer / 'Cargo.toml'), '--filter-platform', 'wasm32-unknown-unknown', '--format-version', '1'], text=True, capture_output=True)
            if result.returncode:
                errors.append(f'{consumer}: cargo metadata failed: {result.stderr.strip()}')
            else:
                errors.extend(check_resolution(json.loads(result.stdout)['packages'], packages, consumer, metadata=True))
    if errors:
        raise SystemExit('\n'.join(errors))
    print(f'OK: {len(packages)} maintained packages match recorded imports; both consumer locks select local sources.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['import', 'verify'])
    parser.add_argument('--output', type=Path, help='Import into a fresh directory for reproducibility checks.')
    parser.add_argument('--seed', type=Path, help='Original provenance.json when rebuilding after removal of the vendor directory.')
    parser.add_argument('--check-metadata', action='store_true', help='Also validate actual Cargo source paths (requires cached external dependencies).')
    args = parser.parse_args()
    if args.output:
        if args.command != 'import':
            parser.error('--output is only supported for import')
        DEST = args.output.resolve()
        PROVENANCE = DEST / 'provenance.json'
    import_archives(args.seed) if args.command == 'import' else verify(args.check_metadata)
