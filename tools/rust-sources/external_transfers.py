"""Exact per-origin external-test transfers for reviewed source consolidations.

A transfer is not a prefix grant: every historical source/hash/origin is named.
Initial transfers reverse unique bounded edits against the last-current hash.
Future reviewed test edits retain attribution and the immutable transfer receipt.
"""
import base64
from collections import Counter
import hashlib
import os
import re
from pathlib import Path

import external_tests

MAX_FILES = 2048
MAX_BYTES = 64 * 1024 * 1024
MAX_EDITS = 512
MAX_PATCH_BYTES = 2 * 1024 * 1024


def digest(data):
    return hashlib.sha256(data).hexdigest()


def label(row, owner):
    return (row.get('origin_directory', owner['origin_directory']), row['source'], row['source_sha256_before'])


def grants(records):
    result = {}
    for record in records.values():
        destination = record['destination_directory']
        allowed = result.setdefault(destination, set())
        for item in record['inputs']:
            owner = item['external_tests']
            if owner:
                allowed.update(label(row, owner) for row in owner['files'])
    return result


def permits(record_grants, directory, row, primary_origin):
    return (row.get('origin_directory', primary_origin), row['source'], row['source_sha256_before']) in record_grants.get(directory, set())


def _patches(edits):
    if not isinstance(edits, list):
        raise ValueError('invalid external transfer replacements')
    result = []
    for edit in edits:
        if not isinstance(edit, dict) or set(edit) != {'old', 'new'}:
            raise ValueError('invalid external transfer replacement')
        if any(not isinstance(edit[k], str) or len(edit[k]) > 2 * MAX_PATCH_BYTES for k in ('old', 'new')):
            raise ValueError('external transfer encoded patch bound exceeded')
        try:
            old, new = (base64.b64decode(edit[k], validate=True) for k in ('old', 'new'))
        except (ValueError, TypeError) as error:
            raise ValueError('invalid external transfer base64') from error
        if not old or not new or old == new:
            raise ValueError('empty or redundant external transfer patch')
        result.append((old, new))
    return result


def _inverse(data, edits, before_hash):
    for old, new in reversed(edits):
        if len(new) >= len(data) or data.count(new) != 1:
            raise ValueError('full-file or ambiguous external transfer substitution')
        previous = data.replace(new, old)
        if previous.count(old) != 1 or previous.replace(old, new) != data:
            raise ValueError('non-reversible external transfer substitution')
        data = previous
    if digest(data) != before_hash:
        raise ValueError('external transfer does not preserve previous test bytes')


def validate(root, record, check_current=True):
    """Authenticate initial content transfer, or validate a frozen history shape."""
    root = Path(root)
    inputs = record['inputs']
    primary = next(item['package'] for item in inputs if item['package']['directory'] == record['primary_directory'])
    before = {}
    prior_roots = set()
    for item in inputs:
        package, owner = item['package'], item['external_tests']
        if owner is None:
            continue
        import source_history_chain
        external_tests.validate_archived(root, owner, package,
            renames=source_history_chain.renames_for(root, record, package['directory']))
        prior_roots.update(owner['roots'])
        for row in owner['files']:
            if row['path'] in before or row.get('origin_directory', package['origin_directory']) != package['origin_directory']:
                raise ValueError('duplicate or wrong-origin external transfer input')
            before[row['path']] = (row, package['origin_directory'])
    routes, owner = record['external_file_routes'], record['destination_external_tests']
    if not isinstance(routes, dict) or set(routes) != set(before) or len(before) > MAX_FILES:
        raise ValueError('external transfer input inventory mismatch/bound')
    if not before:
        if owner is not None:
            raise ValueError('external transfer cannot introduce unattributed test owner')
        return
    if (not isinstance(owner, dict) or set(owner) != {'directory', 'origin_directory', 'roots', 'files', 'file_count', 'bytes'}
            or owner['directory'] != record['destination_directory']
            or owner['origin_directory'] != primary['origin_directory']
            or not isinstance(owner['files'], list) or owner['file_count'] != len(before)
            or len(owner['files']) != len(before) or type(owner['bytes']) is not int
            or not 0 <= owner['bytes'] <= MAX_BYTES or not isinstance(owner['roots'], list) or not owner['roots']):
        raise ValueError('invalid external transfer destination owner')
    after, roots = {}, []
    for relative in owner['roots']:
        external_tests.safe(root, relative)
        if not relative.startswith('test/') or any(relative == p or relative.startswith(p + '/') or p.startswith(relative + '/') for p in roots):
            raise ValueError('overlapping/unsafe external transfer roots')
        roots.append(relative)
    for row in owner['files']:
        external_tests.safe(root, row['path'])
        if row['path'] in after or not any(row['path'].startswith(p + '/') for p in roots):
            raise ValueError('duplicate or unrooted external transfer destination')
        after[row['path']] = row
    total_edits = total_patch = total_current = 0
    destinations = set()
    for old_path, (old, origin) in before.items():
        route = routes[old_path]
        if not isinstance(route, dict) or set(route) != {'destination', 'replacements'}:
            raise ValueError('invalid external transfer route')
        path = route['destination']; external_tests.safe(root, path)
        if path in destinations or path not in after:
            raise ValueError('external transfer is not a complete bijection')
        destinations.add(path)
        row = after[path]
        expected = dict(old, path=path, sha256=row.get('sha256'), origin_directory=origin)
        if row != expected or not re.fullmatch(r'[0-9a-f]{64}', row.get('sha256', '')):
            raise ValueError('external transfer changed historical attribution or metadata')
        edits = _patches(route['replacements'])
        total_edits += len(edits); total_patch += sum(len(a) + len(b) for a, b in edits)
        if total_edits > MAX_EDITS or total_patch > MAX_PATCH_BYTES:
            raise ValueError('external transfer inverse patch bound exceeded')
        if not edits and row['sha256'] != old['sha256']:
            raise ValueError('external transfer changed bytes without inverse proof')
        if check_current:
            current = external_tests.safe(root, path)
            if not current.is_file() or current.stat().st_size > MAX_BYTES:
                raise ValueError('missing/oversized external transfer file')
            with current.open('rb') as stream:
                data = stream.read(MAX_BYTES + 1)
            total_current += len(data)
            if total_current > MAX_BYTES or digest(data) != row['sha256']:
                raise ValueError('external transfer current content drift/bound')
            _inverse(data, edits, old['sha256'])
            if old_path != path and old_path not in after and external_tests.safe(root, old_path).exists():
                raise ValueError('old external transfer file remains')
    if destinations != set(after):
        raise ValueError('external transfer destination inventory mismatch')
    if check_current:
        actual = _inventory(root, roots)
        if actual != {p: row['sha256'] for p, row in after.items()} or total_current != owner['bytes']:
            raise ValueError('external transfer exact live inventory/byte count differs')
    # Absence is a persistent boundary, including nonconventional old roots;
    # unlike historical content hashes it does not freeze current test edits.
    for relative in prior_roots - set(roots):
        if external_tests.safe(root, relative).exists():
            raise ValueError('old external transfer root remains')


def check_current(current, changed):
    for record in changed.values():
        if current.get(record['destination_directory']) != record['destination_external_tests']:
            raise ValueError('external transfer current owner differs from reviewed plan')


def _inventory(root, roots):
    files = {}; total = 0
    for relative in roots:
        directory = external_tests.safe(root, relative)
        if not directory.is_dir():
            raise ValueError('missing external transfer destination root')
        for parent, dirs, names in os.walk(directory, followlinks=False):
            for name in dirs + names:
                path = Path(parent) / name
                p = path.relative_to(root).as_posix()
                external_tests.safe(root, p)
                if path.is_dir():
                    continue
                if not path.is_file() or p in files:
                    raise ValueError('special/duplicate external transfer file')
                total += path.stat().st_size
                files[p] = path
                if len(files) > MAX_FILES or total > MAX_BYTES:
                    raise ValueError('external transfer inventory bound exceeded')
    result = {}
    for p, path in files.items():
        with path.open('rb') as stream:
            data = stream.read(MAX_BYTES + 1)
        if len(data) > MAX_BYTES:
            raise ValueError('external transfer live read bound exceeded')
        result[p] = digest(data)
    return result
