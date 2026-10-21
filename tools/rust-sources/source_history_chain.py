"""Exact, bounded links from ordinary rename/move history into consolidation.

Historical file inventories are not current inventories. The consolidation input
is the exact last-reviewed current snapshot; each earlier receipt remains intact.
"""
from copy import deepcopy

import source_naming
import source_moves

MAX_LINKS = 64


def _touch(package, directory, origin):
    return package.get('directory') == directory or package.get('origin_directory') == origin


def expected(catalog, inputs, destination):
    """Select all intersecting receipts, never a caller-selected subset."""
    result = {}
    for directory, package in inputs.items():
        origin = package['origin_directory']
        renames, moves = {}, {}
        for old, row in catalog.get('renames', {}).items():
            if (row.get('destination_directory') == directory
                    or _touch(row.get('package', {}), directory, origin)):
                renames[old] = deepcopy(row)
            elif destination not in inputs and (old == destination or row.get('destination_directory') == destination):
                raise ValueError('consolidation destination intersects unrelated rename history')
        for key, row in catalog.get('source_moves', {}).items():
            if row.get('directory') == directory or _touch(row.get('package', {}), directory, origin):
                moves[key] = deepcopy(row)
            elif destination not in inputs and row.get('directory') == destination:
                raise ValueError('consolidation destination intersects unrelated move history')
        if renames or moves:
            result[directory] = {'renames': renames, 'source_moves': moves}
    return result


def validate(root, record, catalog=None):
    """Validate embedded receipts and stable lineage without old live files."""
    import source_consolidation as consolidation
    inputs = {item['package']['directory']: item for item in record['inputs']}
    links = record.get('input_history', {})
    if not isinstance(links, dict) or not set(links) <= set(inputs):
        raise ValueError('invalid consolidation input history roots')
    count = 0
    for directory, link in links.items():
        if (not isinstance(link, dict) or set(link) != {'renames', 'source_moves'}
                or not isinstance(link['renames'], dict) or not isinstance(link['source_moves'], dict)
                or not (link['renames'] or link['source_moves'])):
            raise ValueError('invalid explicit consolidation input history binding')
        count += len(link['renames']) + len(link['source_moves'])
        if count > MAX_LINKS or len(link['renames']) > 1:
            raise ValueError('consolidation input history exceeds bound or is ambiguous')
        package = inputs[directory]['package']
        tiny = dict(schema=8, packages={directory: package}, retired={},
                    renames=link['renames'], rename_records_sha256=source_naming.hashes(link['renames']),
                    source_moves=link['source_moves'])
        source_naming.validate_history(root, tiny, check_manifests=False)
        source_moves.validate_history(root, tiny)
        for row in link['renames'].values():
            old = row['package']
            if (row['destination_directory'] != directory
                    or row['destination_name'] != package['name']
                    or not consolidation._stable(old, package)):
                raise ValueError('consolidation rename input lineage mismatch')
        for row in link['source_moves'].values():
            old = row['package']
            if (row['directory'] != directory or row['merged_packages']
                    or old['name'] != package['name']
                    or not consolidation._stable(old, package)):
                raise ValueError('consolidation move input lineage mismatch')
    if catalog is not None:
        source_moves.validate_history(root, catalog)
        if source_naming.hashes(catalog.get('renames', {})) != catalog.get('rename_records_sha256', {}):
            raise ValueError('immutable source rename receipt drift')
        wanted = expected(catalog, {k: v['package'] for k, v in inputs.items()}, record['destination_directory'])
        if links != wanted:
            raise ValueError('consolidation input history differs from exact immutable catalog receipts')
    return links


def renames_for(root, record, directory):
    links = validate(root, record)
    return links.get(directory, {}).get('renames', {})


def consumed_successor(root, catalog, directory, rename, check_manifests=True):
    """Authenticate an absent renamed successor via its consolidation snapshot."""
    import source_consolidation as consolidation
    import external_transfers
    records = consolidation.validate_history(root, catalog, check_manifests=check_manifests,
                                              validate_external=external_transfers.validate)
    found = []
    for record in records.values():
        for item in record['inputs']:
            package = item['package']
            if package['directory'] != directory:
                continue
            links = validate(root, record, catalog)
            if rename not in links.get(directory, {}).get('renames', {}).values():
                raise ValueError('consumed rename lacks exact authenticated history binding')
            found.append(package)
    if len(found) != 1:
        raise ValueError('rename lacks unique authenticated consolidated successor')
    return found[0]
