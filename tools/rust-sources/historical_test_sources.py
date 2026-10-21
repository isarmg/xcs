"""Authenticated source-routing reversal for immutable platform evidence."""
import hashlib
import json
from pathlib import Path
import sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from external_tests import safe, inventory

RECEIPT = 'compliance/platform-test-relocation.json'
RECEIPT_SHA256 = '6adb981c33cc4058bf9225072fe24a66047ef17f7020348d23152e75fb7169ca'


def historical_reader(root):
    content = safe(root, RECEIPT).read_bytes()
    if hashlib.sha256(content).hexdigest() != RECEIPT_SHA256:
        raise ValueError('immutable platform test relocation receipt changed')
    receipt = json.loads(content)
    if receipt['schema_version'] != 1:
        raise ValueError('unexpected platform test relocation schema')
    for relative, expected in receipt['roots'].items():
        if inventory(root, relative) != expected:
            raise ValueError(f'platform external test source inventory/content drift: {relative}')
    before = {}
    for row in receipt['files']:
        source, path = row['source'], row['path']
        safe(root, source)
        if source in before:
            raise ValueError('duplicate platform historical source')
        current = safe(root, path).read_bytes()
        if hashlib.sha256(current).hexdigest() != row['sha256']:
            raise ValueError(f'platform current source content drift: {path}')
        if 'replacement' in row:
            old, new = (row['replacement'][key].encode() for key in ('old', 'new'))
            if not new or current.count(new) != 1:
                raise ValueError('platform source routing is not uniquely reversible')
            previous = current.replace(new, old)
            if previous.count(old) != 1 or previous.replace(old, new) != current:
                raise ValueError('platform source routing is not reversible')
        else:
            previous = current
        if hashlib.sha256(previous).hexdigest() != row['before_sha256']:
            raise ValueError(f'platform historical source content drift: {source}')
        before[source] = previous

    def read(relative):
        safe(root, relative)
        return before[relative] if relative in before else safe(root, relative).read_bytes()
    return read
