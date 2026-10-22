"""Restore validation-only sources from complete, catalog-authenticated archives."""
import base64
import gzip
import hashlib
import json
from pathlib import Path


def require(condition, message):
    if not condition:
        raise ValueError(message)


def restore(root, destination, names):
    root, destination = Path(root), Path(destination).resolve()
    catalog = json.loads((root / 'compliance/rust-sources.json').read_text())
    pending = []
    for name in names:
        owner = 'src/foundation/' + name
        package = dict(catalog['retired'][owner])
        retirement = package.pop('retirement')
        archive = root / f"compliance/{name}-{package['version']}-retirement.json.gz"
        archived = json.loads(gzip.decompress(archive.read_bytes()))
        require(archived['schema_version'] == 1, f'archive schema drift: {owner}')
        require(archived['package'] == package, f'archived package receipt drift: {owner}')
        require(archived['external_tests'] == retirement['external_tests'],
                f'archived external-test receipt drift: {owner}')
        require(archived['retirement'] == {
            key: retirement[key] for key in ('bytes', 'category', 'file_count', 'reason')
        }, f'archived retirement receipt drift: {owner}')
        expected = {owner + '/' + path: digest for path, digest in package['files_sha256'].items()}
        external = archived['external_tests']
        if external:
            expected.update({row['path']: row['sha256'] for row in external['files']})
        require(set(archived['files']) == set(expected), f'archived file inventory drift: {owner}')
        production_bytes = external_bytes = 0
        for relative, encoded in archived['files'].items():
            data = base64.b64decode(encoded, validate=True)
            require(base64.b64encode(data).decode('ascii') == encoded,
                    f'noncanonical archive encoding: {relative}')
            require(hashlib.sha256(data).hexdigest() == expected[relative],
                    f'archived file digest drift: {relative}')
            target = (destination / relative).resolve()
            require(target.is_relative_to(destination), f'archive path escapes destination: {relative}')
            pending.append((target, data))
            if relative.startswith(owner + '/'):
                production_bytes += len(data)
            else:
                external_bytes += len(data)
        require(production_bytes == retirement['bytes'], f'archived source byte count drift: {owner}')
        require(len(package['files_sha256']) == retirement['file_count'],
                f'archived source file count drift: {owner}')
        require(external_bytes == (external['bytes'] if external else 0),
                f'archived external byte count drift: {owner}')
    # Reject any corrupt input before materializing even a partial oracle.
    for target, data in pending:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
