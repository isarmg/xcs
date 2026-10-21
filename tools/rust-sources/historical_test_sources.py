"""Authenticated source-routing reversal for immutable platform evidence."""
import hashlib
import json
from pathlib import Path
import sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from external_tests import safe, inventory

RECEIPT = 'compliance/platform-test-relocation.json'
RECEIPT_SHA256 = '6adb981c33cc4058bf9225072fe24a66047ef17f7020348d23152e75fb7169ca'


CORRECTIONS = 'compliance/platform-test-routing-corrections.json'
CORRECTIONS_SHA256 = 'fca4cd432cb6a51db9bd74a33764339b3e1fb347b7ddb5d950e5b5020acee458'


def corrected_view(root):
    """Reverse only authenticated subsequent routing fixes, preserving old receipts."""
    content = safe(root, CORRECTIONS).read_bytes()
    if hashlib.sha256(content).hexdigest() != CORRECTIONS_SHA256:
        raise ValueError('immutable platform routing correction receipt changed')
    receipt = json.loads(content)
    if set(receipt) != {'schema_version', 'files'} or receipt['schema_version'] != 1:
        raise ValueError('unexpected platform routing correction schema')
    before, paths = {}, set()
    for row in receipt['files']:
        source, path = row['source'], row['path']
        safe(root, source)
        if source in before or path in paths:
            raise ValueError('duplicate platform routing correction')
        paths.add(path)
        current = safe(root, path).read_bytes()
        if hashlib.sha256(current).hexdigest() != row['sha256']:
            raise ValueError(f'platform corrected source content drift: {path}')
        previous = current
        if source != path:
            if safe(root, source).exists() or 'replacement' in row:
                raise ValueError('platform moved fixture must be an exact absent-source move')
        elif 'replacement' in row:
            old, new = (row['replacement'][key].encode() for key in ('old', 'new'))
            if not old or not new or current.count(new) != 1:
                raise ValueError('platform routing correction is not uniquely reversible')
            previous = current.replace(new, old)
            if previous.count(old) != 1 or previous.replace(old, new) != current:
                raise ValueError('platform routing correction is not reversible')
        if hashlib.sha256(previous).hexdigest() != row['before_sha256']:
            raise ValueError('platform routing correction historical content drift')
        before[source] = previous
    return before, receipt['files']


def historical_reader(root):
    content = safe(root, RECEIPT).read_bytes()
    if hashlib.sha256(content).hexdigest() != RECEIPT_SHA256:
        raise ValueError('immutable platform test relocation receipt changed')
    receipt = json.loads(content)
    if receipt['schema_version'] != 1:
        raise ValueError('unexpected platform test relocation schema')
    corrected, corrections = corrected_view(root)
    for relative, expected in receipt['roots'].items():
        actual = inventory(root, relative)
        for row in corrections:
            if row['path'].startswith(relative + '/'):
                del actual[row['path']]
            if row['source'].startswith(relative + '/'):
                actual[row['source']] = row['before_sha256']
        if actual != expected:
            raise ValueError(f'platform external test source inventory/content drift: {relative}')
    before = dict(corrected)
    for row in receipt['files']:
        source, path = row['source'], row['path']
        safe(root, source)
        if source in before:
            raise ValueError('duplicate platform historical source')
        current = corrected[path] if path in corrected else safe(root, path).read_bytes()
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
