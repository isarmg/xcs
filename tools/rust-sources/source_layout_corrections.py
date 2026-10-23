"""Append-only, owner-resolved corrections to external manifest-relative paths."""
from copy import deepcopy
import gzip
import hashlib
import io
import json
import posixpath
import re
from pathlib import Path

KEY = 'foundation_external_path_receipt'
PINNED_RECEIPTS = ('a7d658d10acc05a07c033017e1931e0efcdd73fffc3e124df5ffa701c260e891',)
MAX_BYTES = 2 * 1024 * 1024
_EXPRESSION = re.compile(rb'(?<![A-Za-z0-9_:])(?:core::)?concat!\s*\(\s*(?:core::)?env!\s*\(\s*"CARGO_MANIFEST_DIR"\s*\)\s*,\s*"([^"\n]*)"\s*,?\s*\)')


class _Expressions:
    def finditer(self, data):
        if b'CARGO_MANIFEST_DIR' not in data:
            return
        # Only tokens in Rust code may certify an edit. Quoted/raw literals and
        # nested/line comments are opaque, even if they spell a matching macro.
        code = bytearray(b'\1') * len(data)
        raw_pattern = re.compile(rb'(?:b|c)?r(#+)?"')
        char_pattern = re.compile(rb"'(?:\\.|[^'\\\n])'")
        index = 0
        while index < len(data):
            end = None
            if data.startswith(b'//', index):
                end = data.find(b'\n', index)
                if end < 0:
                    end = len(data)
            elif data.startswith(b'/*', index):
                depth = 1
                end = index + 2
                while end < len(data) and depth:
                    if data.startswith(b'/*', end):
                        depth += 1; end += 2
                    elif data.startswith(b'*/', end):
                        depth -= 1; end += 2
                    else:
                        end += 1
            else:
                raw = raw_pattern.match(data, index)
                if raw:
                    close = b'"' + (raw[1] or b'')
                    position = data.find(close, raw.end())
                    end = len(data) if position < 0 else position + len(close)
                elif data[index:index + 1] == b'"':
                    end = index + 1
                    while end < len(data):
                        if data[end:end + 1] == b'\\':
                            end += 2
                        elif data[end:end + 1] == b'"':
                            end += 1; break
                        else:
                            end += 1
                elif data[index:index + 1] == b"'":
                    char = char_pattern.match(data, index)
                    if char:
                        end = char.end()
            if end is None:
                index += 1
            else:
                code[index:end] = b'\0' * (end - index)
                index = end
        for match in _EXPRESSION.finditer(data):
            if code[match.start()]:
                yield match


EXPRESSION = _Expressions()


def digest(data):
    return hashlib.sha256(data).hexdigest()


def receipt_path(receipt_id):
    if not isinstance(receipt_id, str) or not re.fullmatch('[0-9a-f]{64}', receipt_id):
        raise ValueError('invalid external path correction receipt ID')
    return 'compliance/source-layout-corrections/' + receipt_id + '.json.gz'


def read_receipt(root, receipt_id):
    from source_layout import safe
    if receipt_id not in PINNED_RECEIPTS:
        raise ValueError('unregistered external path correction receipt')
    path = safe(Path(root), receipt_path(receipt_id))
    if not path.is_file() or path.stat().st_size > MAX_BYTES:
        raise ValueError('missing or oversized external path correction receipt')
    raw = path.read_bytes()
    if digest(raw) != receipt_id:
        raise ValueError('immutable external path correction receipt drift')
    with gzip.GzipFile(fileobj=io.BytesIO(raw)) as stream:
        data = stream.read(MAX_BYTES + 1)
        if len(data) > MAX_BYTES or stream.read(1):
            raise ValueError('external path correction expansion bound exceeded')
    return json.loads(data)


def target_for(layout, catalog, owner, literal):
    """Use an exact manifest root and authenticated consolidation file routes."""
    from source_layout import safe
    if owner not in layout.reverse or not isinstance(literal, str) or not literal.startswith('/'):
        raise ValueError('external path correction requires a declared owner and concat suffix')
    target = posixpath.normpath(layout.reverse[owner] + '/' + literal.lstrip('/'))
    safe(layout.root, target)
    seen = set()
    while True:
        destinations = {record['external_file_routes'][target]['destination']
                        for record in catalog.get('consolidations', {}).values()
                        if target in record.get('external_file_routes', {})
                        and record['external_file_routes'][target]['destination'] != target}
        if not destinations:
            return target
        if len(destinations) != 1 or target in seen:
            raise ValueError('ambiguous or cyclic external test target route')
        seen.add(target)
        target = destinations.pop()
        safe(layout.root, target)


class Corrections:
    def __init__(self, root, layout, catalog, receipt, receipt_id):
        from source_layout import safe, HEX
        self.root = Path(root).resolve()
        self.layout = layout
        self.receipt = receipt
        self.receipt_id = receipt_id
        self.rows = {}
        self.owners = set()
        if (not isinstance(receipt, dict)
                or set(receipt) != {'schema', 'layout_receipt_id', 'predecessor_catalog_sha256', 'files'}
                or type(receipt['schema']) is not int or receipt['schema'] != 1
                or receipt['layout_receipt_id'] != layout.receipt_id
                or not isinstance(receipt['predecessor_catalog_sha256'], str)
                or not HEX.fullmatch(receipt['predecessor_catalog_sha256'])
                or not isinstance(receipt['files'], list) or not 1 <= len(receipt['files']) <= 512):
            raise ValueError('invalid external path correction receipt schema')
        for row in receipt['files']:
            if not isinstance(row, dict) or set(row) != {'path', 'owner', 'before_sha256', 'after_sha256', 'before_size', 'after_size', 'references'}:
                raise ValueError('invalid external path correction file fields')
            path, owner = row['path'], row['owner']
            safe(self.root, path)
            if not path.startswith('test/foundation/') or path in self.rows or owner not in layout.reverse:
                raise ValueError('unknown or duplicate external path correction ownership')
            original = layout.rows[layout.reverse[owner]]['destination_external_tests']
            entries = [] if original is None else [entry for entry in original['files'] if entry['path'] == path]
            if len(entries) != 1 or entries[0]['sha256'] != row['before_sha256']:
                raise ValueError('external path correction predecessor is not the immutable owner inventory')
            if (any(not isinstance(row[key], str) or not HEX.fullmatch(row[key]) for key in ('before_sha256', 'after_sha256'))
                    or row['before_sha256'] == row['after_sha256']
                    or any(type(row[key]) is not int or not 0 < row[key] <= MAX_BYTES for key in ('before_size', 'after_size'))
                    or not isinstance(row['references'], list) or not 1 <= len(row['references']) <= 1024):
                raise ValueError('invalid external path correction content bounds')
            self.rows[path] = row
            self.owners.add(owner)
            live = safe(self.root, path)
            if not live.is_file() or live.stat().st_size > MAX_BYTES:
                raise ValueError('missing or oversized corrected external file: ' + path)
            before = self.read(path, live.read_bytes())
            matches = {match.span(1): match[1].decode('utf-8') for match in EXPRESSION.finditer(before)}
            destinations = {entry['path'] for entry in original['files']}
            for ref in row['references']:
                if matches.get((ref['start'], ref['end'])) != ref['before']:
                    raise ValueError('external correction is not an exact manifest expression literal')
                expected = target_for(layout, catalog, owner, ref['before'])
                wanted = '/' + posixpath.relpath(expected, owner)
                if (ref['target'] != expected or ref['after'] != wanted or expected not in destinations
                        or posixpath.normpath(owner + '/' + ref['after'].lstrip('/')) != expected
                        or not safe(self.root, expected).is_file()):
                    raise ValueError('external correction changed or lost its exact owner-attributed target')
        self.validate_catalog(catalog)

    def read(self, path, data):
        row = self.rows.get(path)
        if row is None:
            return data
        if len(data) != row['after_size'] or digest(data) != row['after_sha256']:
            raise ValueError('corrected external live bytes drift: ' + path)
        offsets = []
        delta = 0
        end = 0
        for ref in row['references']:
            if (not isinstance(ref, dict) or set(ref) != {'start', 'end', 'before', 'after', 'target'}
                    or type(ref['start']) is not int or type(ref['end']) is not int
                    or not end <= ref['start'] < ref['end'] <= row['before_size']
                    or any(not isinstance(ref[key], str) for key in ('before', 'after', 'target'))):
                raise ValueError('invalid bounded external correction reference')
            old, new = ref['before'].encode(), ref['after'].encode()
            if len(old) != ref['end'] - ref['start'] or old == new:
                raise ValueError('invalid external correction literal lengths')
            offsets.append((ref['start'] + delta, old, new))
            delta += len(new) - len(old)
            end = ref['end']
        if row['before_size'] + delta != row['after_size']:
            raise ValueError('external correction size mismatch')
        for start, old, new in reversed(offsets):
            if data[start:start + len(new)] != new:
                raise ValueError('external correction inverse context drift')
            data = data[:start] + old + data[start + len(new):]
        if len(data) != row['before_size'] or digest(data) != row['before_sha256']:
            raise ValueError('external correction predecessor bytes drift')
        return data

    def project_hash(self, path, sha):
        row = self.rows.get(path)
        return row['before_sha256'] if row is not None and sha == row['after_sha256'] else sha

    def delta(self, path, sha):
        row = self.rows.get(path)
        return row['after_size'] - row['before_size'] if row is not None and sha == row['after_sha256'] else 0

    def validate_catalog(self, catalog):
        if catalog.get(KEY) != self.receipt_id:
            raise ValueError('external path correction catalog binding drift')
        attributed = {}
        for owner, record in catalog.get('external_tests', {}).items():
            for entry in record['files']:
                path = entry['path']
                if path in attributed:
                    raise ValueError('duplicate external path attribution')
                attributed[path] = (owner, entry)
        for path, row in self.rows.items():
            owner, entry = attributed.get(path, (None, {}))
            if row['owner'] not in catalog.get('external_tests', {}):
                raise ValueError('corrected external owner lifecycle requires a descendant adapter')
            if owner != row['owner'] or entry.get('sha256') != row['after_sha256']:
                raise ValueError('corrected external file owner or current hash drift')


def load(root, layout, catalog):
    from source_layout import safe
    marker = catalog.get(KEY)
    if marker is None:
        if any(safe(Path(root), receipt_path(key)).exists() for key in PINNED_RECEIPTS):
            raise ValueError('external correction receipt present but catalog binding removed')
        return None
    return Corrections(root, layout, catalog, read_receipt(root, marker), marker)


def prepare(root, previous, plan, previous_sha256):
    import source_layout
    if (not isinstance(plan, dict) or set(plan) != {'schema', 'receipt_id'} or plan['schema'] != 1
            or KEY in previous):
        raise ValueError('external path correction requires a separate new explicit plan')
    receipt = read_receipt(root, plan['receipt_id'])
    if receipt['predecessor_catalog_sha256'] != previous_sha256:
        raise ValueError('stale external path correction plan')
    current = deepcopy(previous)
    current[KEY] = plan['receipt_id']
    for row in receipt['files']:
        owner = current.get('external_tests', {}).get(row['owner'])
        entries = [] if owner is None else [entry for entry in owner['files'] if entry['path'] == row['path']]
        if len(entries) != 1 or entries[0]['sha256'] != row['before_sha256']:
            raise ValueError('external correction installation predecessor drift')
        entries[0]['sha256'] = row['after_sha256']
        owner['bytes'] += row['after_size'] - row['before_size']
    layout = source_layout.load(root, current)
    if layout is None:
        raise ValueError('external path correction requires an authenticated foundation layout')
    return layout, current
