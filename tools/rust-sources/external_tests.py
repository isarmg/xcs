"""Exact, owner-scoped current test material accounting, separate from origins."""
import hashlib
import json
import os
import re
from pathlib import Path

MAPPINGS = 'compliance/external-tests'


def safe(root, relative):
    if (not isinstance(relative, str) or not relative or '\\' in relative
            or any(p in ('', '.', '..') or ':' in p for p in relative.split('/'))):
        raise ValueError(f'unsafe external test path: {relative}')
    current = root
    for part in relative.split('/'):
        current /= part
        if current.is_symlink():
            raise ValueError(f'external test symlink: {relative}')
    if not current.resolve().is_relative_to(root.resolve()):
        raise ValueError(f'external test escape: {relative}')
    return current


def inventory(root, relative):
    directory = safe(root, relative)
    if not directory.is_dir():
        raise ValueError(f'missing external test root: {relative}')
    result = {}
    for base, dirs, names in os.walk(directory, followlinks=False):
        for name in dirs + names:
            path = Path(base) / name
            rel = path.relative_to(root).as_posix()
            safe(root, rel)
            if path.is_file():
                result[rel] = hashlib.sha256(path.read_bytes()).hexdigest()
            elif not path.is_dir():
                raise ValueError(f'external test special file: {rel}')
    return result


def load(root, packages):
    """Never learn new source bytes during refresh: verify explicit receipts first."""
    directory = safe(root, MAPPINGS)
    if not directory.exists():
        return {}, {}
    owners, receipts, all_roots, paths = {}, {}, [], set()
    for file in sorted(directory.iterdir()):
        safe(root, file.relative_to(root).as_posix())
        if not file.is_file() or file.suffix != '.json':
            raise ValueError(f'unexpected external test mapping: {file}')
        content = file.read_bytes()
        receipts[file.relative_to(root).as_posix()] = hashlib.sha256(content).hexdigest()
        mapping = json.loads(content)
        if mapping.get('schema_version') != 1:
            raise ValueError('unsupported external test mapping schema')
        for owner in mapping['owners']:
            identity = owner['directory']
            if identity not in packages or identity in owners:
                raise ValueError(f'unknown/duplicate external test owner: {identity}')
            expected = {}
            for entry in owner['files']:
                source, path = entry['source'], entry['path']
                if not re.fullmatch(r'[0-9a-f]{64}', entry.get('source_sha256_before', '')):
                    raise ValueError(f'missing original test source hash: {source}')
                safe(root, source)
                safe(root, path)
                if not source.startswith(identity + '/') or not path.startswith('test/') or path in paths:
                    raise ValueError(f'invalid external test ownership: {path}')
                paths.add(path)
                expected[path] = entry['sha256']
            actual = {}
            for relative in owner['roots']:
                safe(root, relative)
                if not relative.startswith('test/') or any(relative == p or relative.startswith(p + '/') or p.startswith(relative + '/') for p in all_roots):
                    raise ValueError(f'overlapping/unsafe external test roots: {relative}')
                all_roots.append(relative)
                actual.update(inventory(root, relative))
            if actual != expected:
                raise ValueError(f'external test inventory/content drift: {identity}')
            owners[identity] = dict(owner, origin_directory=packages[identity]['origin_directory'],
                                   file_count=len(expected), bytes=sum(safe(root, p).stat().st_size for p in expected))
    # Any new file beneath an attributed foundation package must be inventoried,
    # including files placed next to (rather than inside) a declared subdirectory.
    foundation = root / 'test/foundation'
    if foundation.exists():
        actual = inventory(root, 'test/foundation')
        if set(actual) != {p for p in paths if p.startswith('test/foundation/')}:
            raise ValueError('unattributed external foundation test material')
    return owners, receipts


def check_removed(previous, current, owners):
    for directory, package in previous.items():
        if directory not in current or package.get('origin_directory') != current[directory].get('origin_directory'):
            continue
        removed = set(package.get('files_sha256', {})) - set(current[directory]['files_sha256'])
        attributed = {entry['source'][len(directory) + 1:] for entry in owners.get(directory, {}).get('files', [])
                      if package.get('files_sha256', {}).get(entry['source'][len(directory) + 1:]) == entry['source_sha256_before']}
        # Prior external receipts remain explicit even if a test is relocated again.
        if not removed <= attributed:
            raise ValueError(f'unaccounted source removal during test separation: {directory}: {sorted(removed - attributed)}')


def check_previous(previous, current):
    """Relocation may rename destinations, but must not silently lose fragments."""
    from collections import Counter
    for directory, owner in previous.items():
        old = Counter((row['source'], row['source_sha256_before']) for row in owner['files'])
        new = Counter((row['source'], row['source_sha256_before']) for row in current.get(directory, {}).get('files', []))
        if old - new:
            raise ValueError(f'external test source accounting shrank: {directory}')
