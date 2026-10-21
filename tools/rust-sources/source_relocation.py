"""Pinned, reversible initial test separation; generic refresh cannot bless loss."""
import base64
import gzip
import hashlib
import json

RECEIPT = 'compliance/test-source-relocation.json.gz'
RECEIPT_SHA256 = '791ad728a408677dd54ef98e3feaffe825c5fc5670e888440615933d9498ecc3'
MAX_EXPANDED = 256 * 1024 * 1024


def read_receipt(root, safe):
    path = safe(root, RECEIPT)
    if path.stat().st_size > 16 * 1024 * 1024:
        raise ValueError('compressed test relocation receipt exceeds bound')
    content = path.read_bytes()
    if hashlib.sha256(content).hexdigest() != RECEIPT_SHA256:
        raise ValueError('test source relocation receipt changed')
    with gzip.GzipFile(fileobj=__import__('io').BytesIO(content)) as stream:
        expanded = stream.read(MAX_EXPANDED + 1)
        if len(expanded) > MAX_EXPANDED or stream.read(1):
            raise ValueError('test source relocation receipt exceeds bound')
    receipt = json.loads(expanded)
    if set(receipt) != {'schema_version', 'mapping', 'sources'} or receipt['schema_version'] != 1:
        raise ValueError('unexpected source relocation schema')
    return receipt


def verify(root, owners, safe, baseline):
    receipt = read_receipt(root, safe)
    expected = {(row['source'], row['path'], row['source_sha256_before'], row['sha256'])
                for owner in owners.values() for row in owner['files']}
    if len(expected) != len(receipt['mapping']) or expected != {tuple(row) for row in receipt['mapping']}:
        raise ValueError('test source relocation mapping changed; explicit change receipt required')
    sources = {row[0] for row in expected}
    if sources != set(receipt['sources']):
        raise ValueError('test source relocation source inventory changed')
    for source, record in receipt['sources'].items():
        if set(record) != {'before_sha256', 'outputs', 'reverse_edits'}:
            raise ValueError('unexpected test relocation record fields')
        if {row[2] for row in expected if row[0] == source} != {record['before_sha256']}:
            raise ValueError(f'test source relocation baseline mismatch: {source}')
        if baseline.get(source) != record['before_sha256']:
            raise ValueError(f'test relocation source differs from previous catalog: {source}')
        wanted_outputs = {row[1] for row in expected if row[0] == source}
        if safe(root, source).exists():
            wanted_outputs.add(source)
        actual_outputs = [row['path'] for row in record['outputs']]
        if len(actual_outputs) != len(set(actual_outputs)) or set(actual_outputs) != wanted_outputs:
            raise ValueError(f'test relocation output inventory changed: {source}')
        bundle = b''
        for output in record['outputs']:
            if set(output) != {'path', 'sha256'}:
                raise ValueError('unexpected test relocation output fields')
            current = safe(root, output['path']).read_bytes()
            if hashlib.sha256(current).hexdigest() != output['sha256']:
                raise ValueError(f'test relocation current content changed: {output["path"]}; explicit change receipt required')
            bundle += current
        lines = bundle.splitlines(keepends=True)
        last_start = len(lines) + 1
        for edit in reversed(record['reverse_edits']):
            if set(edit) != {'start', 'end', 'current_sha256', 'original'}:
                raise ValueError('unexpected test relocation edit fields')
            start, end = edit['start'], edit['end']
            if not 0 <= start <= end <= len(lines) or end > last_start:
                raise ValueError('invalid test relocation reverse edit')
            last_start = start
            chunk = b''.join(lines[start:end])
            if hashlib.sha256(chunk).hexdigest() != edit['current_sha256']:
                raise ValueError('test relocation reverse edit context changed')
            lines[start:end] = [base64.b64decode(edit['original'], validate=True)]
        if hashlib.sha256(b''.join(lines)).hexdigest() != record['before_sha256']:
            raise ValueError(f'test relocation cannot reconstruct baseline: {source}')
    return receipt
