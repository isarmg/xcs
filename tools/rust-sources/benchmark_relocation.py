"""Pinned, additive post-consolidation whole-file benchmark separation.

This deliberately supports one reviewed transaction, not a general migration
language. The old consolidation/initial-separation proofs remain unmodified.
No grant is usable until every row, frozen proof and current byte is checked.
"""
from copy import deepcopy
import hashlib
import json
import tomllib

import external_tests
import source_consolidation

RECEIPT = '8a08cc5b47c335b9e6aa2b7361ab7259fdc394c8dad16c29efb3e90d330ffbbc'
PATH = 'compliance/benchmark-relocation/' + RECEIPT + '.json'
KEY = 'post_consolidation_benchmark_relocation'
SCOPES = {'src/foundation/date-time': ('benchmarks', 17),
          'src/foundation/schema-validation': ('benches', 15)}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def read(root):
    if root.is_symlink():
        raise ValueError('benchmark relocation repository root is a symlink')
    path = external_tests.safe(root, PATH)
    if not path.is_file() or path.stat().st_size > 1024 * 1024:
        raise ValueError('missing/oversized benchmark relocation evidence')
    raw = path.read_bytes()
    if digest(raw) != RECEIPT:
        raise ValueError('immutable benchmark relocation receipt drift')
    receipt = json.loads(raw)
    if (receipt['schema'] != 1 or len(receipt['packages']) != 2
            or {p['directory'] for p in receipt['packages']} != set(SCOPES)):
        raise ValueError('benchmark relocation scope mismatch')
    for relative, expected in receipt['frozen_evidence'].items():
        if digest(external_tests.safe(root, relative).read_bytes()) != expected:
            raise ValueError('benchmark relocation frozen predecessor evidence drift')
    return receipt


def validate(root, catalog, inventory, *, installing=False):
    """Return exact grants only after independently checking the full transaction."""
    relevant = (KEY in catalog or external_tests.safe(root, PATH).exists() or any(directory in catalog.get('packages', {})
        or directory in catalog.get('external_tests', {})
        or any(p.get('directory') == directory for p in catalog.get('merges', []))
        or any(p.get('destination_directory') == directory for p in catalog.get('consolidations', {}).values())
        or external_tests.safe(root, directory).exists()
        or external_tests.safe(root, 'test/' + directory[len('src/'):]).exists()
        for directory in SCOPES))
    if not relevant and not installing:
        return None, {}
    marker = catalog.get(KEY)
    if not installing and marker != RECEIPT:
        raise ValueError('missing or changed benchmark relocation catalog evidence')
    if installing and marker is not None:
        raise ValueError('benchmark relocation already installed')
    receipt = read(root)
    grants = {}
    for item in receipt['packages']:
        directory = item['directory']; subtree, count = SCOPES[directory]
        before = item['before_package']; after = item['proposed_files_sha256']
        consolidation = catalog.get('consolidations', {}).get(item['consolidation_id'])
        if (consolidation is None or source_consolidation.receipt_id(consolidation) != item['consolidation_id']
                or consolidation['destination_directory'] != directory):
            raise ValueError('benchmark relocation consolidation predecessor mismatch')
        current = catalog['packages'].get(directory)
        merges = [p for p in catalog.get('merges', []) if p['directory'] == directory]
        if installing:
            if (current != before or merges != item['before_merged_packages']
                    or catalog['external_tests'].get(directory) != item['before_external_tests']):
                raise ValueError('benchmark relocation stale package/merged/external snapshot')
        else:
            # Production implementation bytes may evolve under ordinary refresh.
            # Preserve identities and every shared merged origin, not old code hashes.
            def identity(package):
                return {k: v for k, v in package.items() if k not in ('files_sha256', 'changes')}
            if (current is None or identity(current) != identity(before)
                    or [identity(p) for p in merges] != [identity(p) for p in item['before_merged_packages']]):
                raise ValueError('benchmark relocation current origin identity drift')
            owner = catalog['external_tests'].get(directory, {})
            from collections import Counter
            def labels(rows):
                return Counter((r.get('origin_directory'), r['source'], r['source_sha256_before']) for r in rows)
            if labels(item['before_external_tests']['files']) - labels(owner.get('files', [])):
                raise ValueError('benchmark relocation historical external attribution lost')
            for row in item['proposed_external_tests']['files'][len(item['before_external_tests']['files']):]:
                if owner.get('files', []).count(row) != 1:
                    raise ValueError('benchmark relocation exact retained benchmark row missing')
        manifest = external_tests.safe(root, directory + '/Cargo.toml').read_bytes()
        cargo = tomllib.loads(manifest.decode())
        if ((installing and digest(manifest) != item['manifest_sha256']) or cargo['package'].get('autobenches') is not False or cargo.get('bench')):
            raise ValueError('benchmark relocation must not activate or edit manifests')
        if installing and inventory(external_tests.safe(root, directory)) != after:
            raise ValueError('benchmark relocation production inventory drift or partial move')
        if external_tests.safe(root, directory + '/' + subtree).exists():
            raise ValueError('benchmark relocation old source directory remains')
        rows = item['routes']; origins = {p['package']['directory']: p['package'] for p in consolidation['inputs']}
        if len(rows) != count or len({r['source'] for r in rows}) != count or len({r['destination'] for r in rows}) != count:
            raise ValueError('benchmark relocation non-bijective file routes')
        remaining = dict(before['files_sha256']); additions = []; total = 0
        for row in rows:
            source, destination = row['source'], row['destination']
            external_tests.safe(root, source); target = external_tests.safe(root, destination)
            if (not source.startswith(directory + '/' + subtree + '/') or not source.endswith('.rs')
                    or destination != 'test/' + source[len('src/'):]):
                raise ValueError('benchmark relocation route outside reviewed scope')
            relative = source[len(directory) + 1:]
            if remaining.pop(relative, None) != row['sha256'] or row['sha256'] != row['source_sha256_before']:
                raise ValueError('benchmark relocation changed whole-file bytes')
            historical = []
            for old, routes in consolidation['file_routes'].items():
                for rel, route in routes.items():
                    if route == {'destination': relative}:
                        historical.append((old + '/' + rel, origins[old]['origin_directory'], origins[old]['files_sha256'][rel]))
            if historical != [(row['historical_source'], row['origin_directory'], row['sha256'])]:
                raise ValueError('benchmark relocation historical route/hash/origin mismatch')
            if not target.is_file() or target.stat().st_size != row['bytes'] or digest(target.read_bytes()) != row['sha256']:
                raise ValueError('benchmark relocation missing or changed destination')
            total += row['bytes']
            addition = {'source': row['historical_source'], 'path': destination, 'sha256': row['sha256'],
                        'source_sha256_before': row['sha256'], 'origin_directory': row['origin_directory']}
            additions.append(addition)
            grants.setdefault(directory, set()).add((row['origin_directory'], row['historical_source'], row['sha256']))
        owner = item['proposed_external_tests']; old_owner = item['before_external_tests']
        if (remaining != after or total != item['bytes'] or item['count'] != count
                or owner['files'] != old_owner['files'] + additions
                or owner['file_count'] != old_owner['file_count'] + count
                or owner['bytes'] != old_owner['bytes'] + total
                or owner['roots'][:len(old_owner['roots'])] != old_owner['roots']):
            raise ValueError('benchmark relocation incomplete combined inventory accounting')
        if not installing:
            continue
        roots = owner['roots']; actual = {}
        for index, relative in enumerate(roots):
            if any(relative == r or relative.startswith(r + '/') or r.startswith(relative + '/') for r in roots[:index]):
                raise ValueError('benchmark relocation overlapping owner roots')
            actual.update(external_tests.inventory(root, relative))
        if actual != {r['path']: r['sha256'] for r in owner['files']} or sum(external_tests.safe(root,p).stat().st_size for p in actual) != owner['bytes']:
            raise ValueError('benchmark relocation external inventory/bytes drift')
    return receipt, grants


def prepare(root, previous, installing, inventory):
    return validate(root, previous, inventory, installing=installing)


def combine_grants(existing, additions):
    result = deepcopy(existing)
    for directory, rows in additions.items():
        result.setdefault(directory, set()).update(rows)
    return result


def remap_previous(previous, receipt, installing):
    """Normalize only the exact newly authenticated removals, never future drops."""
    result = deepcopy(previous)
    if installing and receipt is not None:
        for item in receipt['packages']:
            directory = item['directory']
            if result.get(directory) != item['before_package']:
                raise ValueError('benchmark relocation prior removal owner mismatch')
            result[directory]['files_sha256'] = deepcopy(item['proposed_files_sha256'])
    return result


def check_current_external(current, receipt, installing):
    """At installation, existing rows and roots must survive unchanged."""
    if installing:
        for item in receipt['packages']:
            if current.get(item['directory']) != item['proposed_external_tests']:
                raise ValueError('benchmark relocation loaded external owner differs from exact plan')
