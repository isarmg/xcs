"""Authenticated source-routing reversal for immutable platform evidence."""
import hashlib
import base64
import gzip
import io
import json
from pathlib import Path
import sys
sys.path.insert(0, str(Path(__file__).resolve().parent))
from external_tests import safe, inventory
import proof_routing

RECEIPT = 'compliance/platform-test-relocation.json'
RECEIPT_SHA256 = '6adb981c33cc4058bf9225072fe24a66047ef17f7020348d23152e75fb7169ca'


CORRECTIONS = 'compliance/platform-test-routing-corrections.json'
CORRECTIONS_SHA256 = 'fca4cd432cb6a51db9bd74a33764339b3e1fb347b7ddb5d950e5b5020acee458'


def corrected_view(root, view=None):
    view = view or proof_routing.load(root)
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
        current = view.read(path)
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


# Only these explicitly reviewed retired identities may supply historical bytes.
RETIREMENT_ARCHIVES = {
    'src/foundation/getrandom-0.2.17': ('compliance/getrandom-0.2.17-retirement.json.gz', 'f3d9f7987cbb0961c094e884a33938ab3684079d8c23a595c85eca906c36a304'),
    'src/foundation/getrandom-0.3.4': (
        'compliance/getrandom-0.3.4-retirement.json.gz', 'd77bfee293adbab44ad01ebb7991f88ad6a13dd0a98833110269215aa52ceae5'),
}
MAX_RETIREMENT_COMPRESSED = 256 * 1024
MAX_RETIREMENT_EXPANDED = 1024 * 1024


def retired_view(root):
    if not RETIREMENT_ARCHIVES:
        return {}, {}
    catalog = json.loads(safe(root, 'compliance/rust-sources.json').read_bytes())
    archived, roots = {}, {}
    for directory, (receipt_path, pinned) in RETIREMENT_ARCHIVES.items():
        path = safe(root, receipt_path)
        if path.stat().st_size > MAX_RETIREMENT_COMPRESSED:
            raise ValueError('retired platform archive compressed bound exceeded')
        content = path.read_bytes()
        if hashlib.sha256(content).hexdigest() != pinned:
            raise ValueError('immutable retired platform archive changed')
        with gzip.GzipFile(fileobj=io.BytesIO(content)) as stream:
            content = stream.read(MAX_RETIREMENT_EXPANDED + 1)
            if len(content) > MAX_RETIREMENT_EXPANDED or stream.read(1):
                raise ValueError('retired platform archive expanded bound exceeded')
        receipt = json.loads(content)
        if (not isinstance(receipt, dict)
                or set(receipt) != {'schema_version', 'package', 'external_tests', 'retirement', 'files'}
                or type(receipt['schema_version']) is not int or receipt['schema_version'] != 1
                or not isinstance(receipt['package'], dict) or not isinstance(receipt['external_tests'], dict)
                or not isinstance(receipt['files'], dict) or not 1 <= len(receipt['files']) <= 128
                or not isinstance(receipt['package'].get('files_sha256'), dict)
                or not receipt['package']['files_sha256']):
            raise ValueError('invalid retired platform archive schema')
        package, owner = receipt['package'], receipt['external_tests']
        retired = catalog.get('retired', {}).get(directory)
        if (not isinstance(retired, dict) or package.get('directory') != directory
                or {k: v for k, v in retired.items() if k != 'retirement'} != package
                or directory in catalog.get('packages', {})
                or directory in catalog.get('external_tests', {})
                or any(m.get('directory') == directory for m in catalog.get('merges', []))):
            raise ValueError('retired platform archive identity/last receipt mismatch')
        record_hash = hashlib.sha256(json.dumps(retired, sort_keys=True, ensure_ascii=False).encode()).hexdigest()
        if catalog.get('retired_records_sha256', {}).get(directory) != record_hash:
            raise ValueError('immutable retired platform catalog receipt changed')
        retirement = retired.get('retirement', {})
        if retirement != receipt['retirement']:
            raise ValueError('retired platform retirement receipt mismatch')
        if retirement.get('external_tests') != owner:
            raise ValueError('retired platform external receipt mismatch')
        if safe(root, directory).exists():
            raise ValueError('retired platform production root has returned')
        from external_tests import validate_archived
        validate_archived(root, owner, package, removed=True)
        expected = {directory + '/' + p: h for p, h in package['files_sha256'].items()}
        production = set(expected)
        for row in owner['files']:
            if row['path'] in expected:
                raise ValueError('overlapping retired platform inventory')
            expected[row['path']] = row['sha256']
        if set(receipt['files']) != set(expected):
            raise ValueError('retired platform archive inventory mismatch')
        decoded = {}
        for relative, value in receipt['files'].items():
            safe(root, relative)
            if not isinstance(value, str):
                raise ValueError('invalid retired platform byte encoding')
            data = base64.b64decode(value, validate=True)
            if hashlib.sha256(data).hexdigest() != expected[relative]:
                raise ValueError('retired platform archived content mismatch')
            decoded[relative] = data
        if (retirement.get('file_count') != len(production)
                or retirement.get('bytes') != sum(len(decoded[p]) for p in production)
                or owner['bytes'] != sum(len(decoded[row['path']]) for row in owner['files'])):
            raise ValueError('retired platform archive count/byte mismatch')
        for relative in owner['roots']:
            if relative in roots:
                raise ValueError('duplicate retired platform external root')
            roots[relative] = {p: h for p, h in expected.items() if p.startswith(relative + '/')}
        if set(archived) & set(decoded):
            raise ValueError('duplicate retired platform archived path')
        archived.update(decoded)
    return archived, roots


def historical_reader(root):
    content = safe(root, RECEIPT).read_bytes()
    if hashlib.sha256(content).hexdigest() != RECEIPT_SHA256:
        raise ValueError('immutable platform test relocation receipt changed')
    receipt = json.loads(content)
    if receipt['schema_version'] != 1:
        raise ValueError('unexpected platform test relocation schema')
    view = proof_routing.load(root)
    corrected, corrections = corrected_view(root, view)
    archived, archived_roots = retired_view(root)
    for relative, expected in receipt['roots'].items():
        actual = dict(archived_roots[relative]) if relative in archived_roots else view.inventory(relative)
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
        current = corrected[path] if path in corrected else archived[path] if path in archived else view.read(path)
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
        return before[relative] if relative in before else archived[relative] if relative in archived else view.read(relative)
    return read
