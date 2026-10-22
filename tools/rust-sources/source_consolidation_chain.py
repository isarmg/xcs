"""Bounded receipt DAG for repeated consolidation of exact physical owners.

Edges reference immutable receipts, not names or mutable current file hashes.
Transient projections carry every origin; they never replace historical records.
Unsupported legacy merges, upgrade histories and pinned views stay explicit errors.
"""
from collections import Counter
from copy import deepcopy

MAX_DEPTH = 64
MAX_ORIGINS = 4096


def links(record):
    return record.get('input_consolidations', {})


def snapshots(record):
    for item in record['inputs']:
        yield item['package']
        yield from links(record).get(item['package']['directory'], {}).get('merged_packages', [])


def _labels(owner):
    import external_transfers
    try:
        return Counter(external_transfers.label(row, owner) for row in owner['files']) if owner else Counter()
    except (KeyError, TypeError) as error:
        raise ValueError('incomplete nested consolidation external snapshot') from error


def analyze(root, records):
    """Validate graph and return output projections, consumed IDs and input grants.

    Callers authenticate IDs/hashes and the ordinary record structure first.
    Every receipt output is consumed at most once. Independent leaf identities
    and physical roots cannot overlap; directory reuse is only an explicit edge.
    """
    import source_consolidation as c
    import external_transfers
    consumed, dependencies, output, grants, depths = {}, {}, {}, {}, {}
    leaves, leaf_directories = set(), set()
    for key, record in records.items():
        bound = links(record)
        inputs = {item['package']['directory']: item for item in record['inputs']}
        if not isinstance(bound, dict) or not set(bound) <= set(inputs):
            raise ValueError('invalid nested consolidation input roots')
        dependencies[key] = set()
        for directory, link in bound.items():
            if (not isinstance(link, dict) or set(link) != {'receipt_id', 'merged_packages'}
                    or not isinstance(link['receipt_id'], str) or link['receipt_id'] not in records
                    or not isinstance(link['merged_packages'], list)
                    or not 1 <= len(link['merged_packages']) < MAX_ORIGINS):
                raise ValueError('nested consolidation requires an exact predecessor and merged snapshots')
            prior = link['receipt_id']
            if prior == key or prior in consumed:
                raise ValueError('duplicate or cyclic nested consolidation predecessor')
            consumed[prior] = key
            dependencies[key].add(prior)
        for directory, item in inputs.items():
            if directory not in bound:
                origin = item['package']['origin_directory']
                if origin in leaves or directory in leaf_directories:
                    raise ValueError('duplicate or unlinked consolidation input history')
                leaves.add(origin); leaf_directories.add(directory)
    pending = set(records)
    while pending:
        ready = sorted(key for key in pending if dependencies[key] <= output.keys())
        if not ready:
            raise ValueError('cyclic nested consolidation history')
        for key in ready:
            record, origins, row_grants = records[key], {}, {}
            depth = 1 + max((depths[p] for p in dependencies[key]), default=0)
            if depth > MAX_DEPTH:
                raise ValueError('nested consolidation depth bound exceeded')
            for item in record['inputs']:
                package = item['package']; directory = package['directory']
                bound = links(record).get(directory)
                members = [package]
                if bound is not None:
                    prior_key = bound['receipt_id']; prior = records[prior_key]
                    prior_primary = output[prior_key][next(p['package']['origin_directory'] for p in prior['inputs']
                        if p['package']['directory'] == prior['primary_directory'])]
                    if (directory != prior['destination_directory'] or package['name'] != prior['destination_name']
                            or not c._stable(prior_primary, package)):
                        raise ValueError('nested consolidation primary predecessor lineage mismatch')
                    if c._has_build_script_archive(prior):
                        raise ValueError('nested build retirement requires a separate chain-aware adapter')
                    members += bound['merged_packages']
                    inherited = {}
                    for merged in bound['merged_packages']:
                        c._package(root, merged)
                        origin = merged['origin_directory']
                        if (origin in inherited or origin == package['origin_directory']
                                or merged['directory'] != directory
                                or merged.get('merged_into') != directory
                                or merged['files_sha256'] != package['files_sha256']):
                            raise ValueError('nested consolidation duplicate origin or shared input inventory drift')
                        inherited[origin] = merged
                    expected = {origin: row for origin, row in output[prior_key].items()
                                if origin != package['origin_directory']}
                    if set(inherited) != set(expected) or any(
                            not c._stable(row, inherited[origin]) or row['name'] != inherited[origin]['name']
                            for origin, row in expected.items()):
                        raise ValueError('nested consolidation merged origin coverage or identity drift')
                    previous_external = prior['destination_external_tests']
                    current_external = item['external_tests']
                    if _labels(previous_external) - _labels(current_external):
                        raise ValueError('nested consolidation external origin accounting shrank')
                    row_grants[directory] = external_transfers.grants({prior_key: prior}).get(directory, set())
                for member in members:
                    origin = member['origin_directory']
                    if origin in origins:
                        raise ValueError('duplicate nested consolidation origin')
                    # Read-only projection: do not duplicate every large inventory
                    # once per inherited origin or recursively embed old receipts.
                    after = dict(member)
                    after.update(directory=record['destination_directory'],
                                 files_sha256=record['destination_files_sha256'])
                    if member is package and directory == record['primary_directory']:
                        after['name'] = record['destination_name']
                    else:
                        after['merged_into'] = record['destination_directory']
                    origins[origin] = after
            if len(origins) > MAX_ORIGINS:
                raise ValueError('nested consolidation origin bound exceeded')
            output[key], grants[key], depths[key] = origins, row_grants, depth
            pending.remove(key)
    # A directory may recur only along its producer's explicitly connected chain.
    uses = {}
    for key, record in records.items():
        for directory in {record['destination_directory'], *[i['package']['directory'] for i in record['inputs']]}:
            uses.setdefault(directory, set()).add(key)
    for keys in uses.values():
        tips = keys - {key for key in keys if consumed.get(key) in keys}
        if len(tips) != 1:
            raise ValueError('unlinked or reused consolidation physical root')
    terminals = set(records) - consumed.keys()
    terminal_origins = set()
    for key in terminals:
        if terminal_origins & output[key].keys():
            raise ValueError('duplicate terminal consolidation origin coverage')
        terminal_origins.update(output[key])
    return output, consumed, grants


def expected(previous, inputs, records, consumed):
    """Exact last-current bound owners for plan construction/verification."""
    result = {}
    terminal = set(records) - consumed.keys()
    for directory in inputs:
        candidates = [key for key in terminal if records[key]['destination_directory'] == directory]
        merged = [deepcopy(row) for row in previous.get('merges', []) if row.get('directory') == directory]
        if len(candidates) > 1:
            raise ValueError('ambiguous nested consolidation predecessor')
        if candidates:
            result[directory] = dict(receipt_id=candidates[0], merged_packages=merged)
        elif merged:
            raise ValueError('consolidating already merged legacy inputs requires a chain-aware adapter')
    return result
