"""Reviewed multi-origin consolidations with immutable, content-addressed proofs.

This helper never writes files. External ownership is deliberately not authorized
by a broad source-prefix exception: the caller supplies validate_external, whose
contract is (root, record, check_current=True). Nested receipts additionally
receive exact input_grants from their authenticated predecessor graph. It must
validate historical attribution/transfers, and live bytes only for check_current.
"""
import base64
import binascii
from copy import deepcopy
import hashlib
import json
from pathlib import Path
import posixpath
import re
import tomllib

import external_tests
import source_history_chain
import source_consolidation_chain as chain

SCHEMA = 8
HEX = re.compile(r'[0-9a-f]{64}')
NAME = re.compile(r'[a-z][a-z0-9]*(?:-[a-z0-9]+)*')
FIELDS = {'primary_directory', 'destination_directory', 'destination_name', 'reason',
          'inputs', 'destination_files_sha256', 'file_routes', 'new_files',
          'destination_external_tests', 'external_file_routes'}
STABLE = ('origin_directory', 'upstream', 'version', 'license', 'license_materials',
          'repository', 'vcs')
MAX_INPUTS = 64
MAX_FILES = 100000
MAX_RECORDS = 1024
MAX_ARCHIVE_BYTES = 1024 * 1024
MAX_SOURCE_ARCHIVES = 64
MAX_SOURCE_ARCHIVE_TOTAL = 4 * 1024 * 1024
MAX_RECEIPT_BYTES = 32 * 1024 * 1024
MAX_PATH_BYTES = 1024


def _encoded(record):
    try:
        value = json.dumps(record, sort_keys=True, ensure_ascii=False, allow_nan=False).encode('utf-8')
    except (TypeError, ValueError, UnicodeError, RecursionError) as error:
        raise ValueError('invalid consolidation JSON receipt') from error
    if len(value) > MAX_RECEIPT_BYTES:
        raise ValueError('consolidation receipt exceeds size bound')
    return value


def receipt_id(record):
    return hashlib.sha256(_encoded(record)).hexdigest()


def hashes(records):
    if not isinstance(records, dict) or any(not isinstance(key, str) for key in records):
        raise ValueError('invalid consolidation receipt hash mapping')
    return {key: receipt_id(record) for key, record in sorted(records.items())}


def _safe(root, relative):
    if (not isinstance(relative, str) or not relative or relative.startswith('/')
            or len(relative.encode('utf-8')) > MAX_PATH_BYTES
            or any(ord(char) < 32 or ord(char) == 127 for char in relative)):
        raise ValueError('unsafe consolidation path')
    if root.is_symlink():
        raise ValueError('consolidation repository root symlink')
    return external_tests.safe(root, relative)


def _foundation(root, relative):
    path = _safe(root, relative)
    if not relative.startswith('src/foundation/') or len(relative.split('/')) != 3:
        raise ValueError('consolidation requires direct foundation package roots')
    return path


def _inventory(root, value, empty=False):
    if not isinstance(value, dict) or len(value) > MAX_FILES or (not empty and not value):
        raise ValueError('invalid bounded consolidation file inventory')
    for relative, digest in value.items():
        _safe(root, relative)
        if not isinstance(digest, str) or not HEX.fullmatch(digest):
            raise ValueError('invalid consolidation file hash')


def _package(root, package):
    if (not isinstance(package, dict)
            or not {'directory', 'name', 'version', 'origin_directory', 'upstream', 'files_sha256'} <= package.keys()
            or not isinstance(package['name'], str) or not package['name']
            or not isinstance(package['version'], str) or not package['version']
            or not isinstance(package['upstream'], dict) or not package['upstream']):
        raise ValueError('incomplete consolidation input package snapshot')
    _foundation(root, package['directory'])
    _safe(root, package['origin_directory'])
    _inventory(root, package['files_sha256'])
    if 'Cargo.toml' not in package['files_sha256']:
        raise ValueError('consolidation package snapshot lacks root Cargo.toml')
    materials = package.get('license_materials', [])
    if not isinstance(materials, list) or len(materials) > MAX_FILES:
        raise ValueError('invalid consolidation license material paths')
    for path in materials:
        _safe(root, path)


def _archive(route, expected):
    value = route['archive_base64']
    if not isinstance(value, str) or len(value) > 4 * ((MAX_ARCHIVE_BYTES + 2) // 3):
        raise ValueError('archived consolidation manifest exceeds size bound')
    try:
        raw = base64.b64decode(value, validate=True)
    except (ValueError, binascii.Error) as error:
        raise ValueError('invalid archived consolidation manifest base64') from error
    if (not raw or len(raw) > MAX_ARCHIVE_BYTES
            or base64.b64encode(raw).decode('ascii') != value
            or hashlib.sha256(raw).hexdigest() != expected):
        raise ValueError('archived consolidation manifest does not match exact before hash')
    try:
        manifest = tomllib.loads(raw.decode('utf-8'))
    except (ValueError, UnicodeError) as error:
        raise ValueError('archived consolidation manifest is not valid TOML') from error
    if not isinstance(manifest.get('package'), dict):
        raise ValueError('archived consolidation manifest lacks package identity')
    return manifest['package']



def _source_archive(relative, route, expected):
    """Keep removed optional implementation bytes, never discard or test routes."""
    if (route['kind'] != 'archived-source' or not relative.startswith('src/')
            or not relative.endswith('.rs') or relative in ('src/lib.rs', 'src/main.rs')
            or {part.lower().split('.', 1)[0] for part in relative.split('/')} & {
                'test', 'tests', 'bench', 'benches', 'example', 'examples', 'fixture', 'fixtures',
                'legal', 'license', 'licenses', 'licensing', 'copyright', 'notice',
                'provenance', 'compliance'}):
        raise ValueError('source archive requires a non-entrypoint Rust implementation path')
    return _source_archive_bytes(route, expected)


def _source_archive_bytes(route, expected):
    reason = route['reason']
    if not isinstance(reason, str) or not 40 <= len(reason.strip()) <= 10000:
        raise ValueError('source archive requires a substantive reviewed reason')
    if route['sha256'] != expected:
        raise ValueError('source archive hash differs from exact input inventory')
    value = route['archive_base64']
    if not isinstance(value, str) or len(value) > 4 * ((MAX_ARCHIVE_BYTES + 2) // 3):
        raise ValueError('source archive encoded size bound exceeded')
    try:
        raw = base64.b64decode(value, validate=True)
        raw.decode('utf-8')
    except (ValueError, binascii.Error, UnicodeError) as error:
        raise ValueError('source archive requires canonical base64 UTF-8 source bytes') from error
    if (not raw or len(raw) > MAX_ARCHIVE_BYTES
            or base64.b64encode(raw).decode('ascii') != value
            or hashlib.sha256(raw).hexdigest() != expected):
        raise ValueError('source archive bytes differ from exact input hash/bound')
    return len(raw)


def _build_script_archive(relative, route, expected, directory, package, record):
    """Archive only an authenticated secondary build entrypoint into inert evidence."""
    if (route['kind'] != 'archived-build-script' or relative != 'build.rs'
            or directory == record['primary_directory']):
        raise ValueError('build script archive requires a secondary root build.rs')
    manifest_route = record['file_routes'][directory].get('Cargo.toml')
    if not isinstance(manifest_route, dict) or set(manifest_route) != {'archive_base64'}:
        raise ValueError('build script archive requires the exact archived input manifest')
    manifest = _archive(manifest_route, package['files_sha256']['Cargo.toml'])
    if (manifest.get('name') != package['name'] or manifest.get('version') != package['version']
            or manifest.get('build', 'build.rs') != 'build.rs'
            or 'links' in manifest
            or (isinstance(manifest.get('license-file'), str)
                and posixpath.normpath(manifest['license-file']) == 'build.rs')
            or any(posixpath.normpath(path) in ('build.rs', directory + '/build.rs')
                   for path in package.get('license_materials', []))):
        raise ValueError('build script archive input identity/build/legal ownership mismatch')
    after = record['destination_files_sha256']
    destination_manifest = _archive({'archive_base64': route['destination_manifest_base64']},
                                    after['Cargo.toml'])
    if (destination_manifest.get('name') != record['destination_name']
            or destination_manifest.get('build') is not False
            or 'links' in destination_manifest or 'build.rs' in after
            or expected in after.values()):
        raise ValueError('build script archive requires an explicitly inert destination')
    return _source_archive_bytes(route, expected)


def _primary_build_retirement(root, relative, route, expected, directory, package, record):
    """Authenticate a reviewed static replacement, not behavioral equivalence."""
    if (route['kind'] != 'primary-build-script-retirement' or relative != 'build.rs'
            or directory != record['primary_directory']):
        raise ValueError('primary build retirement requires the primary root build.rs')
    if record['file_routes'][directory].get('Cargo.toml') != {'destination': 'Cargo.toml'}:
        raise ValueError('primary build retirement must retain the primary manifest route')
    manifest = _archive({'archive_base64': route['original_manifest_base64']},
                        package['files_sha256']['Cargo.toml'])
    if (manifest.get('name') != package['name'] or manifest.get('version') != package['version']
            or manifest.get('build', 'build.rs') != 'build.rs' or 'links' in manifest
            or (isinstance(manifest.get('license-file'), str)
                and posixpath.normpath(manifest['license-file']) == 'build.rs')
            or any(posixpath.normpath(path) in ('build.rs', directory + '/build.rs')
                   for path in package.get('license_materials', []))):
        raise ValueError('primary build retirement input identity/build/legal ownership mismatch')
    after = record['destination_files_sha256']
    destination = _archive({'archive_base64': route['destination_manifest_base64']}, after['Cargo.toml'])
    if (destination.get('name') != record['destination_name']
            or destination.get('version') != package['version']
            or destination.get('build') is not False or 'links' in destination
            or 'build.rs' in after or expected in after.values()):
        raise ValueError('primary build retirement requires an explicitly inert destination')
    toolchain = route['toolchain']
    if (not isinstance(toolchain, dict) or set(toolchain) != {'rust_version', 'targets', 'assumptions'}
            or toolchain['rust_version'] != '1.99' or destination.get('rust-version') != '1.99'
            or toolchain['targets'] != ['x86_64-unknown-linux-gnu', 'wasm32-unknown-unknown']
            or not isinstance(toolchain['assumptions'], str)
            or not 40 <= len(toolchain['assumptions'].strip()) <= 10000):
        raise ValueError('primary build retirement requires explicit supported toolchain assumptions')
    replacements = route['replacement_sources']
    if (not isinstance(replacements, dict) or not 1 <= len(replacements) <= 16
            or 'src/lib.rs' not in replacements):
        raise ValueError('primary build retirement requires bounded static replacement sources')
    total = 0
    for path, evidence in replacements.items():
        _safe(root, path)
        if (not path.startswith('src/') or not path.endswith('.rs') or path in ('src/main.rs', 'src/build.rs')
                or {part.lower().split('.', 1)[0] for part in path.split('/')} & {
                    'test', 'tests', 'bench', 'benches', 'example', 'examples', 'fixture', 'fixtures',
                    'legal', 'license', 'licenses', 'licensing', 'copyright', 'notice', 'provenance', 'compliance'}
                or path not in after or not isinstance(evidence, dict)
                or set(evidence) != {'sha256', 'archive_base64'}):
            raise ValueError('primary build retirement replacement must be an inventoried Rust production source')
        size = _source_archive_bytes(dict(evidence, reason=route['reason']), after[path])
        total += size
        if size > 256 * 1024 or total > 1024 * 1024:
            raise ValueError('primary build retirement replacement byte bound exceeded')
    return _source_archive_bytes(route, expected)


def _has_build_script_archive(record):
    return any(route.get('kind') in ('archived-build-script', 'primary-build-script-retirement')
               for mapping in record['file_routes'].values() for route in mapping.values())


def _record(root, record):
    if not isinstance(record, dict) or not FIELDS <= set(record) or set(record) - FIELDS - {'input_history', 'input_consolidations'}:
        raise ValueError('invalid explicit consolidation record fields')
    _encoded(record)
    primary, destination = record['primary_directory'], record['destination_directory']
    _foundation(root, primary); _foundation(root, destination)
    if (not isinstance(record['destination_name'], str)
            or not NAME.fullmatch(record['destination_name'])
            or destination.rsplit('/', 1)[1] != record['destination_name']
            or not isinstance(record['reason'], str) or not record['reason'].strip()
            or len(record['reason']) > 10000):
        raise ValueError('invalid consolidation destination identity or reason')
    inputs = record['inputs']
    if not isinstance(inputs, list) or not 2 <= len(inputs) <= MAX_INPUTS:
        raise ValueError('consolidation requires two or more bounded input snapshots')
    packages, origins = {}, set()
    for item in inputs:
        if not isinstance(item, dict) or set(item) != {'package', 'external_tests'}:
            raise ValueError('invalid consolidation input snapshot fields')
        package = item['package']
        _package(root, package)
        directory, origin = package['directory'], package['origin_directory']
        if directory in packages or origin in origins:
            raise ValueError('duplicate consolidation input directory or origin')
        if item['external_tests'] is not None and not isinstance(item['external_tests'], dict):
            raise ValueError('invalid consolidation external input snapshot')
        packages[directory] = package
        origins.add(origin)
    if primary not in packages or (destination in packages and destination != primary):
        raise ValueError('consolidation destination must be new or the selected primary')
    after, routes, new_files = record['destination_files_sha256'], record['file_routes'], record['new_files']
    _inventory(root, after); _inventory(root, new_files, empty=True)
    if 'Cargo.toml' not in after:
        raise ValueError('consolidation destination lacks root Cargo.toml')
    if not isinstance(routes, dict) or set(routes) != set(packages):
        raise ValueError('consolidation production routes must cover exact input directories')
    mapped = set()
    archived_count = archived_bytes = 0
    for directory, package in packages.items():
        mapping = routes[directory]
        if not isinstance(mapping, dict) or set(mapping) != set(package['files_sha256']):
            raise ValueError('consolidation production routes must cover every before file')
        for relative, route in mapping.items():
            if not isinstance(route, dict):
                raise ValueError('invalid consolidation production route')
            if set(route) == {'destination'}:
                target = route['destination']
                _safe(root, target)
                if target not in after:
                    raise ValueError('consolidation production route target missing from destination')
                mapped.add(target)
            elif set(route) == {'kind', 'sha256', 'archive_base64', 'reason', 'destination_manifest_base64',
                                'original_manifest_base64', 'replacement_sources', 'toolchain'}:
                archived_bytes += _primary_build_retirement(root, relative, route, package['files_sha256'][relative],
                                                           directory, package, record)
                archived_count += 1
                if archived_count > MAX_SOURCE_ARCHIVES or archived_bytes > MAX_SOURCE_ARCHIVE_TOTAL:
                    raise ValueError('aggregate source archive bound exceeded')
            elif set(route) == {'kind', 'sha256', 'archive_base64', 'reason', 'destination_manifest_base64'}:
                archived_bytes += _build_script_archive(relative, route, package['files_sha256'][relative],
                                                       directory, package, record)
                archived_count += 1
                if archived_count > MAX_SOURCE_ARCHIVES or archived_bytes > MAX_SOURCE_ARCHIVE_TOTAL:
                    raise ValueError('aggregate source archive bound exceeded')
            elif set(route) == {'kind', 'sha256', 'archive_base64', 'reason'}:
                archived_bytes += _source_archive(relative, route, package['files_sha256'][relative])
                archived_count += 1
                if archived_count > MAX_SOURCE_ARCHIVES or archived_bytes > MAX_SOURCE_ARCHIVE_TOTAL:
                    raise ValueError('aggregate source archive bound exceeded')
            elif set(route) == {'archive_base64'}:
                if relative != 'Cargo.toml' or directory == primary:
                    raise ValueError('only redundant input root Cargo.toml may be archived')
                _archive(route, package['files_sha256'][relative])
            else:
                raise ValueError('consolidation forbids generic production discard dispositions')
    if (mapped & set(new_files) or mapped | set(new_files) != set(after)
            or any(after.get(path) != digest for path, digest in new_files.items())):
        raise ValueError('consolidation destination inventory needs exact disjoint mapped/new coverage')
    if not isinstance(record['external_file_routes'], dict):
        raise ValueError('invalid consolidation external route map')
    if record['destination_external_tests'] is not None and not isinstance(record['destination_external_tests'], dict):
        raise ValueError('invalid consolidation destination external snapshot')
    return packages


def _external(root, record, validator, check_current, input_grants=None):
    has_external = (record['destination_external_tests'] is not None
                    or bool(record['external_file_routes'])
                    or any(row['external_tests'] is not None for row in record['inputs']))
    if validator is None:
        if has_external:
            raise ValueError('consolidation external transfer requires an explicit validator')
    else:
        if chain.links(record):
            validator(root, record, check_current=check_current, input_grants=input_grants)
        else:
            validator(root, record, check_current=check_current)


def _absence(root, record):
    destination = record['destination_directory']
    for item in record['inputs']:
        old = item['package']['directory']
        if old == destination:
            continue
        if _foundation(root, old).exists():
            raise ValueError('old consolidated source root remains or has returned')
        if _safe(root, 'test/foundation/' + old.rsplit('/', 1)[1]).exists():
            raise ValueError('old consolidated external test root remains or has returned')


def _manifest(root, record, primary):
    directory = _foundation(root, record['destination_directory'])
    _safe(root, record['destination_directory'] + '/Cargo.toml')
    try:
        manifest = tomllib.loads((directory / 'Cargo.toml').read_text(encoding='utf-8'))['package']
        version = manifest.get('version')
        if isinstance(version, dict) and version.get('workspace') is True:
            _safe(root, 'Cargo.toml')
            version = tomllib.loads((root / 'Cargo.toml').read_text(encoding='utf-8'))['workspace']['package']['version']
    except (OSError, ValueError, KeyError, TypeError) as error:
        raise ValueError('invalid consolidation destination manifest') from error
    if manifest.get('name') != record['destination_name'] or version != primary['version']:
        raise ValueError('consolidation manifest name/version identity drift')
    if _has_build_script_archive(record) and (manifest.get('build') is not False
            or 'links' in manifest or (directory / 'build.rs').exists()):
        raise ValueError('archived build script destination must remain explicitly inert')


def _origin_rows(catalog):
    packages, merges = catalog.get('packages'), catalog.get('merges', [])
    retired, superseded = catalog.get('retired', {}), catalog.get('superseded', {})
    if (not isinstance(packages, dict) or not isinstance(merges, list)
            or not isinstance(retired, dict) or not isinstance(superseded, dict)):
        raise ValueError('invalid consolidation catalog ownership collections')
    rows = list(packages.values()) + merges + list(retired.values())
    for receipt in superseded.values():
        if not isinstance(receipt, dict) or not isinstance(receipt.get('package'), dict):
            raise ValueError('invalid consolidation catalog superseded owner')
        rows.append(receipt['package'])
    by_origin = {}
    for package in rows:
        if not isinstance(package, dict) or not isinstance(package.get('origin_directory'), str):
            raise ValueError('invalid consolidation catalog origin owner')
        origin = package['origin_directory']
        if origin in by_origin:
            raise ValueError('duplicate current consolidation origin coverage')
        by_origin[origin] = package
    return by_origin


def _stable(before, after):
    return all((key in before) == (key in after) and before.get(key) == after.get(key) for key in STABLE)


def validate_history(root, catalog, check_manifests=True, validate_external=None):
    """Validate historical proofs and current membership, not freeze old bytes."""
    root = Path(root)
    records = catalog.get('consolidations', {})
    if not isinstance(records, dict) or len(records) > MAX_RECORDS:
        raise ValueError('invalid bounded consolidation history')
    if records and (type(catalog.get('schema')) is not int or catalog['schema'] < SCHEMA):
        raise ValueError('source consolidation history requires catalog schema 8')
    if hashes(records) != catalog.get('consolidation_records_sha256', {}):
        raise ValueError('immutable consolidation receipt hash drift')
    if not records:
        return records
    owners = _origin_rows(catalog)
    packages = {}
    for key, record in records.items():
        if not isinstance(key, str) or not HEX.fullmatch(key) or key != receipt_id(record):
            raise ValueError('immutable content-addressed consolidation receipt drift')
        packages[key] = _record(root, record)
    outputs, consumed, grants = chain.analyze(root, records)
    for key, record in records.items():
        _absence(root, record)
        _external(root, record, validate_external, check_current=False, input_grants=grants[key])
        if key in consumed:
            continue
        inputs = packages[key]
        if chain.links(record) and (chain._labels(record['destination_external_tests']) -
                chain._labels(catalog.get('external_tests', {}).get(record['destination_directory']))):
            raise ValueError('nested consolidation current external origin accounting shrank')
        primary, destination = record['primary_directory'], record['destination_directory']
        current = catalog['packages'].get(destination)
        if (not isinstance(current, dict) or current.get('directory') != destination
                or current.get('name') != record['destination_name']
                or not _stable(inputs[primary], current)):
            raise ValueError('consolidation lacks the same-origin active primary successor')
        _inventory(root, current.get('files_sha256'))
        for origin, before in outputs[key].items():
            owner = owners.get(origin)
            if (not isinstance(owner, dict) or owner.get('directory') != destination
                    or not _stable(before, owner)
                    or owner.get('files_sha256') != current['files_sha256']):
                raise ValueError('consolidation current origin coverage or shared inventory drift')
            if origin != current['origin_directory']:
                matches = [row for row in catalog.get('merges', [])
                           if row.get('origin_directory') == origin]
                if (len(matches) != 1 or owner is not matches[0] or owner.get('name') != before['name']
                        or owner.get('merged_into') != destination):
                    raise ValueError('consolidation secondary origin must remain exactly one merged successor')
        if check_manifests:
            _manifest(root, record, inputs[primary])
    terminal_directories = {records[key]['destination_directory'] for key in set(records) - consumed.keys()}
    for record in records.values():
        for item in record['inputs']:
            directory = item['package']['directory']
            if directory in catalog.get('retired', {}):
                raise ValueError('consolidation input identity remains separately registered')
            if directory in catalog['packages'] and directory not in terminal_directories:
                raise ValueError('consolidation input identity remains separately registered')
    source_history_chain.validate_all(root, records.values(), catalog)
    return records


def validate_origins(records, origins):
    """Use the independently authenticated manage.origins(root) mapping."""
    for record in records.values():
        for package in chain.snapshots(record):
            origin = origins.get(package['origin_directory'])
            if origin is None or origin.get('upstream') != package['upstream']:
                raise ValueError('consolidation historical upstream origin mismatch')
            for key in ('license', 'repository', 'vcs'):
                if key in origin and package.get(key) != origin[key]:
                    raise ValueError('consolidation historical legal/upstream identity drift')
            if 'central_license_materials' in origin and package.get('license_materials') != origin['central_license_materials']:
                raise ValueError('consolidation historical license inventory drift')


def _unhandled_history(root, previous, inputs, destination, records, record=None, authorize_pinned=None):
    directories = set(inputs)
    bound = chain.links(record or {})
    origins = {p['origin_directory'] for p in inputs.values()}
    origins.update(p['origin_directory'] for link in bound.values() for p in link['merged_packages'])
    ancestors = set()
    pending = [link['receipt_id'] for link in bound.values()]
    while pending:
        key = pending.pop()
        if key in ancestors:
            continue
        ancestors.add(key)
        pending.extend(link['receipt_id'] for link in chain.links(records[key]).values())
    for key, prior_record in records.items():
        used = {item['package']['directory'] for item in prior_record['inputs']} | {prior_record['destination_directory']}
        if key not in ancestors and used & (directories | {destination}):
            raise ValueError('consolidating prior consolidation inputs requires a chain-aware adapter')
    for row in previous.get('superseded', {}).values():
        if (row.get('package', {}).get('directory') in directories | {destination}
                or row.get('successor_origin_directory') in origins):
            raise ValueError('consolidating upgraded inputs requires a chain-aware adapter')
    import benchmark_relocation
    if previous.get(benchmark_relocation.KEY) and directories & set(benchmark_relocation.SCOPES):
        raise ValueError('consolidating benchmark-pinned inputs requires a chain-aware adapter')
    if bound:
        import proof_routing
        predecessors = {link['receipt_id'] for link in bound.values()}
        for key in previous.get('proof_routing_receipts', []):
            binding = proof_routing._read(root, key)['binding']
            if binding['kind'] == 'consolidation' and receipt_id(binding['record']) in predecessors:
                raise ValueError('nested pinned source requires a new historical-view adapter')
    for receipt in ('compliance/ring-test-relocation.json', 'compliance/platform-test-relocation.json'):
        path = _safe(root, receipt)
        if path.exists():
            content = path.read_text(encoding='utf-8')
            if any(directory in content or 'test/foundation/' + directory.rsplit('/', 1)[1] in content for directory in directories):
                if any(route.get('kind') == 'primary-build-script-retirement'
                       for mapping in (record or {}).get('file_routes', {}).values() for route in mapping.values()):
                    raise ValueError('primary build retirement cannot authorize pinned source')
                if authorize_pinned is None or authorize_pinned(root, previous, record) is not True:
                    raise ValueError('consolidating pinned source requires a historical-view adapter')


def prepare(root, previous, plan, file_inventory, validate_external=None, authorize_pinned=None):
    """Return active, merges, immutable history, and new records by destination.

The plan is schema=1, packages=[record,...], with FIELDS and optional exact
input_history/input_consolidations bindings per record.
The file_inventory callback must reject symlinks and special files. The external
validator is mandatory whenever any external data exists, even for old history.
"""
    root = Path(root)
    history = deepcopy(validate_history(root, previous, check_manifests=False,
                                        validate_external=validate_external))
    active, merges, changed = deepcopy(previous['packages']), deepcopy(previous.get('merges', [])), {}
    if plan is None:
        return active, merges, history, changed
    if (not isinstance(plan, dict) or set(plan) != {'schema', 'packages'}
            or type(plan['schema']) is not int or plan['schema'] != 1
            or not isinstance(plan['packages'], list) or not 1 <= len(plan['packages']) <= MAX_RECORDS):
        raise ValueError('invalid explicit source consolidation plan')
    before_origins = _origin_rows(previous)
    _, consumed, _ = chain.analyze(root, history)
    seen = set()
    for record in plan['packages']:
        inputs = _record(root, record)
        primary, destination = record['primary_directory'], record['destination_directory']
        all_paths = set(inputs) | {destination}
        if seen & all_paths:
            raise ValueError('overlapping or chained source consolidation plan')
        seen.update(all_paths)
        for item in record['inputs']:
            package = item['package']
            directory = package['directory']
            if (previous['packages'].get(directory) != package
                    or previous.get('external_tests', {}).get(directory) != item['external_tests']):
                raise ValueError('consolidation plan does not match exact previous package/external receipt')
        if ((destination in previous['packages'] and destination != primary)
                or destination in previous.get('retired', {})):
            raise ValueError('consolidation destination identity is already registered')
        expected_links = chain.expected(previous, inputs, history, consumed)
        if chain.links(record) != expected_links:
            raise ValueError('nested consolidation needs exact previous merged snapshots and chain-aware binding')
        trial_history = dict(history, **{receipt_id(record): record})
        _, trial_consumed, trial_grants = chain.analyze(root, trial_history)
        source_history_chain.validate(root, record, previous)
        _unhandled_history(root, previous, inputs, destination, history, record, authorize_pinned)
        _absence(root, record)
        destination_path = _foundation(root, destination)
        if not destination_path.is_dir():
            raise ValueError('consolidation requires a present destination source root')
        if file_inventory(destination_path) != record['destination_files_sha256']:
            raise ValueError('consolidated source content differs from reviewed destination inventory')
        _manifest(root, record, inputs[primary])
        _external(root, record, validate_external, check_current=True,
                  input_grants=trial_grants[receipt_id(record)])
        for merged in merges:
            if merged['directory'] in inputs:
                merged.update(directory=destination, merged_into=destination,
                              files_sha256=deepcopy(record['destination_files_sha256']))
        for directory, package in inputs.items():
            del active[directory]
            after = deepcopy(package)
            after.update(directory=destination, files_sha256=deepcopy(record['destination_files_sha256']))
            if directory == primary:
                after['name'] = record['destination_name']
                active[destination] = after
            else:
                after['merged_into'] = destination
                merges.append(after)
        key = receipt_id(record)
        if key in history or len(history) >= MAX_RECORDS:
            raise ValueError('duplicate or excessive consolidation receipt history')
        history[key] = deepcopy(record)
        consumed = trial_consumed
        changed[destination] = deepcopy(record)
    catalog = dict(previous, schema=max(previous.get('schema', 0), SCHEMA), packages=active,
                   merges=merges, consolidations=history, consolidation_records_sha256=hashes(history),
                   external_tests=remap_previous_external(previous.get('external_tests', {}), changed))
    if set(_origin_rows(catalog)) != set(before_origins):
        raise ValueError('consolidation origin coverage changed')
    validate_history(root, catalog, validate_external=validate_external)
    return active, merges, history, changed


def remap_previous_external(previous, changed):
    """Normalize only fresh exact external transitions before accounting checks."""
    result = deepcopy(previous)
    for destination, record in changed.items():
        if destination != record['destination_directory']:
            raise ValueError('consolidation changed-record destination mismatch')
        for item in record['inputs']:
            directory, owner = item['package']['directory'], item['external_tests']
            if result.get(directory) != owner:
                raise ValueError('consolidation previous external owner does not match receipt')
            result.pop(directory, None)
        if destination in result:
            raise ValueError('consolidation external destination already registered')
        if record['destination_external_tests'] is not None:
            result[destination] = deepcopy(record['destination_external_tests'])
    return result


def check_current_external(current, changed):
    for destination, record in changed.items():
        if current.get(destination) != record['destination_external_tests']:
            raise ValueError('current external owner differs from reviewed consolidation')


def remap_previous_packages(previous, changed):
    """Normalize only freshly reviewed production transitions for loss checks.

All before files already have explicit routes or an exact redundant-manifest
archive. Replaying the after snapshot here does not grant future removal rights.
"""
    result = deepcopy(previous)
    for destination, record in changed.items():
        if destination != record['destination_directory']:
            raise ValueError('consolidation changed-record destination mismatch')
        primary = None
        for item in record['inputs']:
            package = item['package']
            directory = package['directory']
            if result.get(directory) != package:
                raise ValueError('consolidation previous package does not match receipt')
            if directory == record['primary_directory']:
                primary = package
            del result[directory]
        if primary is None or destination in result:
            raise ValueError('consolidation previous primary/destination identity mismatch')
        after = deepcopy(primary)
        after.update(directory=destination, name=record['destination_name'],
                     files_sha256=deepcopy(record['destination_files_sha256']))
        result[destination] = after
    return result
