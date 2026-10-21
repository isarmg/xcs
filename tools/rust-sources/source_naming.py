"""Explicit, lossless source-identity transitions; historical evidence is immutable.

The first migration supports package-root/test-root and manifest-name changes.
Relative production filenames change only through explicit bijective file_moves;
source fragments cannot disappear.
Call prepare before loading current external mappings; pass its validated history
into source_prefixes and compare the resulting owners with check_current_external.
"""
from copy import deepcopy
import hashlib
import json
import re
import tomllib

import external_tests

HEX = re.compile(r'[0-9a-f]{64}')
NAME = re.compile(r'[a-z][a-z0-9]*(?:-[a-z0-9]+)*')


def hashes(records):
    return {key: hashlib.sha256(json.dumps(value, sort_keys=True, ensure_ascii=False).encode()).hexdigest()
            for key, value in sorted(records.items())}


def _path(root, value):
    return external_tests.safe(root, value)


def _foundation(root, value):
    path = _path(root, value)
    if not value.startswith('src/foundation/') or len(value.split('/')) != 3:
        raise ValueError('rename requires a direct foundation package')
    return path


def _inventory(value):
    if not isinstance(value, dict) or not value:
        raise ValueError('rename requires a nonempty exact file inventory')
    for path, digest in value.items():
        if (not isinstance(path, str) or path.startswith('/') or '\\' in path
                or any(p in ('', '.', '..') or ':' in p for p in path.split('/'))
                or not isinstance(digest, str) or not HEX.fullmatch(digest)):
            raise ValueError('invalid rename file receipt')


def mapped_inventory(before, after, moves):
    """Validate a complete bijection, including unchanged relative filenames."""
    _inventory(before); _inventory(after)
    if not isinstance(moves, dict):
        raise ValueError('invalid explicit rename file_moves')
    if any(old not in before or not isinstance(new, str) or new not in after or old == new
           for old, new in moves.items()):
        raise ValueError('rename file_moves must name distinct exact inventory paths')
    mapped = [moves.get(path, path) for path in before]
    if len(set(mapped)) != len(mapped) or set(mapped) != set(after):
        raise ValueError('rename production file inventory changed without a bijective file map')
    return {moves.get(path, path): digest for path, digest in before.items()}


def _test_prefix(directory):
    return 'test/foundation/' + directory.rsplit('/', 1)[1]


def _translate(path, old, new):
    if path == old:
        return new
    if path.startswith(old + '/'):
        return new + path[len(old):]
    raise ValueError(f'rename path outside exact old root: {path}')


def _external(root, before, after, package, destination):
    """Validate archived-to-current bijection without rewriting source labels."""
    if before is None:
        if after is not None:
            raise ValueError('rename cannot introduce an external test owner')
        return
    if not isinstance(after, dict):
        raise ValueError('rename cannot discard an external test owner')
    old = package['directory']
    if (not isinstance(before, dict) or before.get('directory') != old
            or before.get('origin_directory') != package['origin_directory']
            or not isinstance(before.get('files'), list) or not before['files']
            or type(before.get('file_count')) is not int
            or before['file_count'] != len(before['files'])
            or not isinstance(before.get('roots'), list) or not before['roots']
            or type(before.get('bytes')) is not int or before['bytes'] < 0):
        raise ValueError('rename historical external owner mismatch')
    source_hashes = {}
    for row in before['files']:
        if (not isinstance(row, dict) or not {'source', 'path', 'sha256', 'source_sha256_before'} <= row.keys()
                or any(not isinstance(row.get(k), str) or not HEX.fullmatch(row[k])
                       for k in ('sha256', 'source_sha256_before'))):
            raise ValueError('invalid historical rename external fragment')
        _path(root, row['source']); _path(root, row['path'])
        if (not row['source'].startswith(old + '/')
                or source_hashes.setdefault(row['source'], row['source_sha256_before']) != row['source_sha256_before']):
            raise ValueError('historical rename external source identity mismatch')
    old_root, new_root = _test_prefix(old), _test_prefix(destination)
    expected = deepcopy(before)
    expected['directory'] = destination
    expected['roots'] = [_translate(p, old_root, new_root) for p in before['roots']]
    for row in expected['files']:
        row['path'] = _translate(row['path'], old_root, new_root)
    # Content edits are allowed only as individually reviewed after hashes.
    if not isinstance(after.get('files'), list) or len(after['files']) != len(expected['files']):
        raise ValueError('rename external fragment inventory changed')
    paths = set()
    for row, current in zip(expected['files'], after['files']):
        if not isinstance(current, dict) or set(current) != set(row):
            raise ValueError('rename external fragment fields changed')
        if not isinstance(current.get('sha256'), str) or not HEX.fullmatch(current['sha256']):
            raise ValueError('invalid renamed external hash')
        row['sha256'] = current['sha256']
        if current != row or row['path'] in paths:
            raise ValueError('rename rewrote historical source or lost external fragment')
        paths.add(row['path'])
        _path(root, row['path']); _path(root, row['source'])
    expected['bytes'] = after.get('bytes')
    if type(expected['bytes']) is not int or expected['bytes'] < 0 or after != expected:
        raise ValueError('rename external identity/count/metadata mismatch')
    roots = expected['roots']
    for index, path in enumerate(roots):
        _path(root, path)
        if any(path == p or path.startswith(p + '/') or p.startswith(path + '/') for p in roots[:index]):
            raise ValueError('overlapping renamed external roots')
    if any(not any(p.startswith(r + '/') for r in roots) for p in paths):
        raise ValueError('renamed external fragment outside owned roots')


def validate_history(root, catalog, check_manifests=True):
    """Authenticate append-only catalog receipts, without requiring old files."""
    records = catalog.get('renames', {})
    if records and (type(catalog.get('schema')) is not int or catalog['schema'] < 5):
        raise ValueError('source rename history requires catalog schema 5 or later')
    if not isinstance(records, dict) or hashes(records) != catalog.get('rename_records_sha256', {}):
        raise ValueError('immutable source rename receipt drift')
    destinations, origins = set(), set()
    for old, record in records.items():
        required = {'package', 'destination_directory', 'destination_name', 'reason',
                    'files_sha256', 'external_tests', 'destination_external_tests'}
        if (not isinstance(record, dict) or not required <= record.keys()
                or set(record) - required - {'file_moves'}):
            raise ValueError('invalid source rename record fields')
        package, new = record['package'], record['destination_directory']
        if not isinstance(package, dict):
            raise ValueError('invalid historical rename package')
        _foundation(root, old); _foundation(root, new)
        if (package.get('directory') != old or new == old or new in destinations
                or new in records or not isinstance(record['destination_name'], str)
                or not NAME.fullmatch(record['destination_name'])
                or new.rsplit('/', 1)[1] != record['destination_name']
                or not isinstance(record['reason'], str) or not record['reason'].strip()):
            raise ValueError('invalid/ambiguous source rename identity')
        origin = package.get('origin_directory')
        if not isinstance(origin, str) or not origin or origin in origins:
            raise ValueError('duplicate/missing historical rename origin')
        origins.add(origin)
        destinations.add(new)
        if _path(root, old).exists():
            raise ValueError('old renamed source root has returned')
        if _path(root, _test_prefix(old)).exists():
            raise ValueError('old external test root remains after rename')
        mapped_inventory(package['files_sha256'], record['files_sha256'], record.get('file_moves', {}))
        _external(root, record['external_tests'], record['destination_external_tests'], package, new)
        current = catalog.get('packages', {}).get(new) or catalog.get('retired', {}).get(new)
        if current is None and catalog.get('consolidations'):
            import source_history_chain
            current = source_history_chain.consumed_successor(
                root, catalog, new, record, check_manifests=check_manifests)
        if current is None or current.get('origin_directory') != package['origin_directory'] or current.get('upstream') != package['upstream']:
            raise ValueError('rename has no same-origin active or retired successor')
        if (current.get('directory') != new or current.get('name') != record['destination_name']
                or (new in catalog.get('packages', {}) and new in catalog.get('retired', {}))):
            raise ValueError('renamed current directory/name identity drift')
        if check_manifests and new in catalog.get('packages', {}):
            manifest = tomllib.loads((_path(root, new) / 'Cargo.toml').read_text())['package']
            if manifest.get('name') != record['destination_name']:
                raise ValueError('renamed manifest name drift; explicit migration required')
        if old in catalog.get('packages', {}) or old in catalog.get('retired', {}):
            raise ValueError('renamed identity remains registered at old path')
    return records


def source_prefixes(records, directory, origin_directory):
    """Use only records returned by prepare/validate_history, never raw mappings."""
    return tuple([directory + '/'] + [old + '/' for old, row in records.items()
                 if row['destination_directory'] == directory
                 and row['package']['origin_directory'] == origin_directory])


def prepare(root, previous, plan, file_inventory):
    """Return (active packages, immutable rename records); never write anything.

file_inventory is manage.files: accepts a directory and returns relative hashes.
Plans use schema=1, packages=[{directory, ...record fields...}]. Previous catalog
records and supplied before owners must match exactly. This deliberately rejects
repeat renames and sources with upgrade histories until their chain model exists.
"""
    records = deepcopy(validate_history(root, previous, check_manifests=False))
    active = deepcopy(previous['packages'])
    if plan is None:
        return active, records
    if not isinstance(plan, dict) or set(plan) != {'schema', 'packages'} or type(plan['schema']) is not int or plan['schema'] != 1 or not isinstance(plan['packages'], list) or not plan['packages']:
        raise ValueError('invalid explicit source rename plan')
    seen = set()
    for item in plan['packages']:
        fields = {'directory', 'package', 'destination_directory', 'destination_name', 'reason',
                  'files_sha256', 'external_tests', 'destination_external_tests'}
        if (not isinstance(item, dict) or not fields <= item.keys()
                or set(item) - fields - {'file_moves'}):
            raise ValueError('invalid explicit source rename fields')
        old, new = item['directory'], item['destination_directory']
        old_path, new_path = _foundation(root, old), _foundation(root, new)
        before = previous['packages'].get(old)
        if before is None or item['package'] != before or item['external_tests'] != previous.get('external_tests', {}).get(old):
            raise ValueError('rename plan does not match exact previous receipt')
        if old in seen or old in records or old in {r['destination_directory'] for r in records.values()} or new in previous['packages'] or new in previous.get('retired', {}) or new in records or new in {r['destination_directory'] for r in records.values()}:
            raise ValueError('duplicate, reused, or chained rename identity')
        seen.add(old)
        if any(r['package']['directory'] == old for r in previous.get('superseded', {}).values()):
            raise ValueError('renaming upgraded source requires chain-aware migration')
        # These proofs authenticate a fixed live view; require an adapter first.
        for receipt in ('compliance/ring-test-relocation.json', 'compliance/platform-test-relocation.json'):
            path = _path(root, receipt)
            if path.exists() and (old in path.read_text() or _test_prefix(old) in path.read_text()):
                import proof_routing
                proof_routing.authorize(root, previous, item)
        if old_path.exists() or not new_path.is_dir():
            raise ValueError('rename requires absent old and present new source root')
        after = deepcopy(before)
        after.update(directory=new, name=item['destination_name'], files_sha256=item['files_sha256'])
        records[old] = {k: deepcopy(v) for k, v in item.items() if k != 'directory'}
        del active[old]
        if new in active:
            raise ValueError('duplicate rename destination')
        active[new] = after
        temporary = dict(previous, schema=5, packages=active, renames=records, rename_records_sha256=hashes(records))
        validate_history(root, temporary)
        if file_inventory(new_path) != item['files_sha256']:
            raise ValueError('renamed source content differs from reviewed plan')
        manifest = tomllib.loads((new_path / 'Cargo.toml').read_text())['package']
        if manifest.get('name') != item['destination_name'] or manifest.get('version') != before['version']:
            raise ValueError('renamed manifest identity/version differs from reviewed plan')
        owner = item['destination_external_tests']
        if owner is not None:
            if _path(root, _test_prefix(old)).exists():
                raise ValueError('old external test root remains after rename')
            actual = {}
            for relative in owner['roots']:
                actual.update(external_tests.inventory(root, relative))
            if actual != {r['path']: r['sha256'] for r in owner['files']}:
                raise ValueError('renamed external content differs from reviewed plan')
            if owner['bytes'] != sum(_path(root, p).stat().st_size for p in actual):
                raise ValueError('renamed external byte count mismatch')
    return active, records


def check_current_external(previous, current, records):
    """New transitions must match the reviewed after owner, not a generic refresh."""
    for old, row in records.items():
        if old not in previous.get('packages', {}):
            continue
        new = row['destination_directory']
        if old in current or current.get(new) != row['destination_external_tests']:
            raise ValueError('current external owner differs from reviewed rename plan')


def remap_previous(previous, records):
    """Normalize only newly transitioned keys for ordinary loss-accounting gates."""
    result = deepcopy(previous)
    for old, row in records.items():
        if old not in result:
            continue
        new = row['destination_directory']
        if new in result:
            raise ValueError('rename previous identity collision')
        value = result.pop(old)
        value['directory'] = new
        if 'files_sha256' in value:
            value['files_sha256'] = mapped_inventory(value['files_sha256'], row['files_sha256'], row.get('file_moves', {}))
        result[new] = value
    return result
