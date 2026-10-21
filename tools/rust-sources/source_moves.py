"""Reviewed identity-preserving internal file moves with append-only receipts."""
from copy import deepcopy
import hashlib
import json
import tomllib

import external_tests
import source_naming

FIELDS = {'directory', 'package', 'merged_packages', 'files_sha256', 'file_moves', 'reason',
          'external_tests', 'destination_external_tests', 'external_file_moves'}


def receipt_id(record):
    return hashlib.sha256(json.dumps(record, sort_keys=True, ensure_ascii=False).encode()).hexdigest()


def _scope(root, directory):
    path = external_tests.safe(root, directory)
    if not directory.startswith(('src/', 'tools/')):
        raise ValueError('internal move requires a registered production source root')
    return path


def _external(root, record):
    before, after = record['external_tests'], record['destination_external_tests']
    moves = record['external_file_moves']
    if not isinstance(moves, dict):
        raise ValueError('invalid external internal-move map')
    if before is None:
        if after is not None or moves:
            raise ValueError('internal move cannot introduce external test owner')
        return
    package = record['package']
    if (not isinstance(before, dict) or not isinstance(after, dict)
            or before.get('directory') != record['directory']
            or before.get('origin_directory') not in {p['origin_directory'] for p in [package, *record['merged_packages']]}
            or not isinstance(before.get('roots'), list) or not before['roots']
            or not isinstance(before.get('files'), list) or not before['files']
            or not isinstance(after.get('files'), list)
            or type(before.get('file_count')) is not int
            or before['file_count'] != len(before['files'])
            or len(after['files']) != len(before['files'])
            or type(before.get('bytes')) is not int or before['bytes'] < 0
            or type(after.get('bytes')) is not int or after['bytes'] < 0):
        raise ValueError('invalid internal-move external owner')
    expected = deepcopy(before)
    expected['bytes'] = after['bytes']
    old_hashes, new_hashes = {}, {}
    source_hashes = {}
    for old, new in zip(before['files'], after['files']):
        if (not isinstance(old, dict) or not isinstance(new, dict)
                or not {'path', 'source', 'sha256', 'source_sha256_before'} <= old.keys()
                or old['path'] in old_hashes or new.get('path') in new_hashes):
            raise ValueError('duplicate/invalid external move fragment')
        external_tests.safe(root, old['source']); external_tests.safe(root, old['path'])
        external_tests.safe(root, new['path'])
        if (not isinstance(old['source_sha256_before'], str)
                or not source_naming.HEX.fullmatch(old['source_sha256_before'])
                or source_hashes.setdefault(old['source'], old['source_sha256_before']) != old['source_sha256_before']):
            raise ValueError('invalid historical external source hash')
        wanted = dict(old, path=moves.get(old['path'], old['path']), sha256=new.get('sha256'))
        if new != wanted:
            raise ValueError('internal move rewrote historical external source')
        old_hashes[old['path']], new_hashes[new['path']] = old['sha256'], new['sha256']
    source_naming.mapped_inventory(old_hashes, new_hashes, moves)
    expected['files'] = deepcopy(after['files'])
    if expected != after:
        raise ValueError('internal move changed external owner identity or roots')
    roots = before['roots']
    for index, path in enumerate(roots):
        external_tests.safe(root, path)
        if (not path.startswith('test/') or any(path == p or path.startswith(p + '/') or p.startswith(path + '/') for p in roots[:index])):
            raise ValueError('invalid internal-move external roots')
    if any(not any(p.startswith(r + '/') for r in roots) for p in [*old_hashes, *new_hashes]):
        raise ValueError('internal external move escaped owned roots')


def _record(root, record):
    if not isinstance(record, dict) or set(record) != FIELDS:
        raise ValueError('invalid internal move receipt fields')
    directory, package = record['directory'], record['package']
    _scope(root, directory)
    if (not isinstance(package, dict) or package.get('directory') != directory
            or not all(package.get(key) for key in ('name', 'version', 'origin_directory', 'upstream'))
            or not isinstance(record['reason'], str) or not record['reason'].strip()
            or not isinstance(record['file_moves'], dict)
            or not (record['file_moves'] or record['external_file_moves'])
            or not isinstance(record['merged_packages'], list)):
        raise ValueError('incomplete internal move identity/reason/map')
    source_naming.mapped_inventory(package['files_sha256'], record['files_sha256'], record['file_moves'])
    _external(root, record)
    origins = {package['origin_directory']}
    for merged in record['merged_packages']:
        if (not isinstance(merged, dict) or merged.get('directory') != directory
                or not merged.get('origin_directory') or merged['origin_directory'] in origins
                or not merged.get('upstream')):
            raise ValueError('invalid overlapping merged source move receipt')
        origins.add(merged['origin_directory'])
        source_naming.mapped_inventory(merged['files_sha256'], record['files_sha256'], record['file_moves'])


def validate_history(root, catalog):
    history = catalog.get('source_moves', {})
    if not isinstance(history, dict):
        raise ValueError('invalid internal move history')
    if history and (type(catalog.get('schema')) is not int or catalog['schema'] < 6):
        raise ValueError('internal move history requires catalog schema 6')
    for key, record in history.items():
        if key != receipt_id(record):
            raise ValueError('immutable internal move receipt drift')
        _record(root, record)
    return history


def validate_origins(history, origins):
    """Call with manage.origins(root), which already authenticates original data."""
    for record in history.values():
        for package in [record['package'], *record['merged_packages']]:
            origin = origins.get(package['origin_directory'])
            if origin is None or origin['upstream'] != package['upstream']:
                raise ValueError('internal move historical origin mismatch')


def prepare(root, previous, plan, file_inventory):
    """Return (active packages, complete immutable history, newly moved roots).

Plans use schema=1/packages records with FIELDS exactly. merged_packages must
contain every exact previous merged receipt sharing the root, including an empty
list when none exist. Caller rejects intersections with rename/upgrade/retire.
"""
    history = deepcopy(validate_history(root, previous))
    active, changed = deepcopy(previous['packages']), {}
    if plan is None:
        return active, history, changed
    if (not isinstance(plan, dict) or set(plan) != {'schema', 'packages'}
            or type(plan['schema']) is not int or plan['schema'] != 1
            or not isinstance(plan['packages'], list) or not plan['packages']):
        raise ValueError('invalid explicit internal move plan')
    for record in plan['packages']:
        _record(root, record)
        directory = record['directory']
        if directory in changed:
            raise ValueError('duplicate internal move source root')
        before = previous['packages'].get(directory)
        overlaps = [p for p in previous.get('merges', []) if p['directory'] == directory]
        if (before is None or before != record['package'] or overlaps != record['merged_packages']
                or record['external_tests'] != previous.get('external_tests', {}).get(directory)):
            raise ValueError('internal move requires exact prior source and merged receipts')
        path = _scope(root, directory)
        if file_inventory(path) != record['files_sha256']:
            raise ValueError('internal move content differs from reviewed after inventory')
        manifest = tomllib.loads((path / 'Cargo.toml').read_text())['package']
        version = manifest.get('version')
        if isinstance(version, dict) and version.get('workspace') is True:
            version = tomllib.loads((root / 'Cargo.toml').read_text())['workspace']['package']['version']
        if manifest.get('name') != before['name'] or version != before['version']:
            raise ValueError('internal move cannot change package name/version identity')
        owner = record['destination_external_tests']
        if owner is not None:
            actual = {}
            for relative in owner['roots']:
                actual.update(external_tests.inventory(root, relative))
            if actual != {r['path']: r['sha256'] for r in owner['files']}:
                raise ValueError('internal move external content differs from reviewed inventory')
            if owner['bytes'] != sum(external_tests.safe(root, p).stat().st_size for p in actual):
                raise ValueError('internal move external byte count mismatch')
        key = receipt_id(record)
        if key in history:
            raise ValueError('internal move receipt already exists')
        history[key] = deepcopy(record)
        changed[directory] = deepcopy(record)
        active[directory]['files_sha256'] = deepcopy(record['files_sha256'])
    return active, history, changed


def remap_previous(previous, changed):
    """Normalize this transition only, never replay already committed moves."""
    result = deepcopy(previous)
    for directory, record in changed.items():
        if directory not in result:
            raise ValueError('internal move previous inventory is missing')
        package = result[directory]
        allowed = [record['package'], *record['merged_packages']]
        if package not in allowed:
            raise ValueError('internal move previous inventory does not match receipt')
        package['files_sha256'] = source_naming.mapped_inventory(
            package['files_sha256'], record['files_sha256'], record['file_moves'])
    return result


def check_current_external(current, changed):
    for directory, record in changed.items():
        if current.get(directory) != record['destination_external_tests']:
            raise ValueError('current external owner differs from reviewed internal move')
