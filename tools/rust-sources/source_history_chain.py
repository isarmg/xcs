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


def expected(catalog, inputs, destination, excluded=()):
    """Select every direct receipt; validated nested inputs retain ancestor links.

    excluded is only for graph-authenticated nested owners, never arbitrary roots.
    Their immutable ancestors are independently checked by validate_all.
    """
    result = {}
    for directory, package in inputs.items():
        if directory in excluded:
            continue
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
    if (not isinstance(links, dict) or not set(links) <= set(inputs)
            or set(links) & set(record.get('input_consolidations', {}))):
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
        _validate_catalog_links(root, (record,), catalog)
    return links


def _validate_catalog_links(root, records, catalog):
    source_moves.validate_history(root, catalog)
    if source_naming.hashes(catalog.get('renames', {})) != catalog.get('rename_records_sha256', {}):
        raise ValueError('immutable source rename receipt drift')
    for record in records:
        inputs = {item['package']['directory']: item['package'] for item in record['inputs']}
        wanted = expected(catalog, inputs, record['destination_directory'],
                          excluded=record.get('input_consolidations', {}))
        if record.get('input_history', {}) != wanted:
            raise ValueError('consolidation input history differs from exact immutable catalog receipts')


def validate_all(root, records, catalog):
    """Authenticate all links with one fresh shared catalog scan per operation."""
    records = tuple(records)
    for record in records:
        validate(root, record)
    _validate_catalog_links(root, records, catalog)


def renames_for(root, record, directory):
    links = validate(root, record)
    return links.get(directory, {}).get('renames', {})


def consumed_successors(root, catalog, renames, check_manifests=True):
    """Authenticate requested consumed renames in one read-only validation pass.

    The caller supplies this operation's complete pending rename set. No result
    is retained across calls: current manifests, paths, receipts, and ownership
    are freshly validated by consolidation.validate_history every time.
    """
    import source_consolidation as consolidation
    import external_transfers
    records = consolidation.validate_history(root, catalog, check_manifests=check_manifests,
                                              validate_external=external_transfers.validate)
    found = {directory: [] for directory in renames}
    for record in records.values():
        matching = [item['package'] for item in record['inputs']
                    if item['package']['directory'] in renames
                    and item['package']['directory'] not in record.get('input_consolidations', {})]
        if not matching:
            continue
        links = validate(root, record, catalog)
        for package in matching:
            directory = package['directory']
            if renames[directory] not in links.get(directory, {}).get('renames', {}).values():
                raise ValueError('consumed rename lacks exact authenticated history binding')
            found[directory].append(package)
    if any(len(packages) != 1 for packages in found.values()):
        raise ValueError('rename lacks unique authenticated consolidated successor')
    return {directory: packages[0] for directory, packages in found.items()}


def consumed_successor(root, catalog, directory, rename, check_manifests=True):
    """Authenticate an absent renamed successor via its consolidation snapshot."""
    return consumed_successors(root, catalog, {directory: rename}, check_manifests)[directory]
