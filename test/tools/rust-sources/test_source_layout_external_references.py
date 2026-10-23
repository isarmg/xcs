"""Compiler includes inside retained external modules use their catalog owner.

This independently scans the retained corpus, not the layout edit receipt: an
omitted file must still be checked. It does not claim every retained test is in
the maintained Cargo test closure, nor validate runtime-only fixture paths.
"""
from collections import defaultdict
from pathlib import Path
import json
import os
import posixpath
import re
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
ROOT = Path(os.environ.get('SOURCE_LAYOUT_REPOSITORY', str(HERE.parents[2] if len(HERE.parents) >= 3 else Path.cwd())))
INCLUDE = re.compile(rb'\binclude(?:_str|_bytes)?!\s*\(\s*(?:core::|std::)?concat!\s*\(\s*(?:core::|std::)?env!\s*\(\s*"CARGO_MANIFEST_DIR"\s*\)\s*,\s*"([^"\\\n]*)"\s*,?\s*\)\s*\)')
RAW = re.compile(rb'(?:br|cr|r)(#{0,32})"')
CHAR = re.compile(rb"'(?:\\(?:x[0-9a-fA-F]{2}|u\{[0-9a-fA-F_]+\}|.)|[^'\\\n])'")


def noncode_spans(data):
    """Identify literal/comment spans, never treating examples as compiler code."""
    i = 0
    spans = []
    while i < len(data):
        start = i
        if data.startswith(b'//', i):
            i = data.find(b'\n', i)
            if i < 0: i = len(data)
        elif data.startswith(b'/*', i):
            depth = 1; i += 2
            while i < len(data) and depth:
                if data.startswith(b'/*', i): depth += 1; i += 2
                elif data.startswith(b'*/', i): depth -= 1; i += 2
                else: i += 1
        elif (m := RAW.match(data, i)) is not None:
            end = data.find(b'"' + m[1], m.end())
            i = len(data) if end < 0 else end + 1 + len(m[1])
        elif data[i:i+1] == b'"':
            i += 1
            while i < len(data):
                if data[i:i+1] == b'\\': i += 2
                elif data[i:i+1] == b'"': i += 1; break
                else: i += 1
        elif (m := CHAR.match(data, i)) is not None:
            i = m.end()
        else:
            i += 1
            continue
        spans.append((start, i))
    return spans


def manifest_includes(data):
    if b'CARGO_MANIFEST_DIR' not in data: return []
    spans = noncode_spans(data)
    return [(m.start(1), m.end(1), m[1].decode()) for m in INCLUDE.finditer(data)
            if not any(a <= m.start() < b for a, b in spans)]


def check_external_includes(root, catalog):
    claims = defaultdict(list)
    for owner, record in catalog.get('external_tests', {}).items():
        for item in record.get('files', []):
            claims[item['path']].append(owner)
    errors = []; checked = []
    for path, owners in sorted(claims.items()):
        if not path.endswith('.rs'): continue
        data = (root/path).read_bytes()
        refs = manifest_includes(data)
        if not refs: continue
        if len(owners) != 1:
            errors.append(f'ambiguous external owner: {path}: {owners}')
            continue
        owner = owners[0]
        if owner not in catalog.get('packages', {}):
            # Retired owners are retained evidence, not current compiler bases.
            continue
        for start, end, literal in refs:
            if not literal.startswith('/') or '\\' in literal:
                errors.append(f'noncanonical manifest concatenation: {path}: {literal}')
                continue
            target = posixpath.normpath(owner + '/' + literal[1:])
            if target == '..' or target.startswith(('../', '/')):
                errors.append(f'target escapes repository: {path}: {literal}')
                continue
            if not (root/target).is_file():
                errors.append(f'missing current owner-relative include: {path}:{data.count(bytes([10]),0,start)+1}: {owner} + {literal} -> {target}')
            checked.append((path, owner, target))
    return errors, checked


class NestedOwnerIncludes(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory(prefix='nested-owner-', dir='/tmp')
        self.addCleanup(temp.cleanup); self.root = Path(temp.name)
        self.path = 'test/foundation/misleading-old-name/modules/deep/nested.rs'
        self.owner = 'src/foundation/concurrency/current-owner'
        self.target = 'test/foundation/misleading-old-name/functions/leaf.rs'
        self.catalog = {'packages': {self.owner: {}}, 'external_tests': {
            self.owner: {'files': [{'path': self.path}]}}}
        (self.root/self.path).parent.mkdir(parents=True)
        (self.root/self.target).parent.mkdir(parents=True)
        (self.root/self.target).write_text('fn leaf() {}\n')

    def write(self, literal):
        (self.root/self.path).write_text('mod deeply_nested { include!(concat!(env!("CARGO_MANIFEST_DIR"), "'+literal+'")); }\n')

    def test_stationary_nested_file_uses_owner_not_file_directory(self):
        self.write('/../../../../'+self.target)
        errors, checked = check_external_includes(self.root,self.catalog)
        self.assertEqual(errors,[])
        self.assertEqual(checked,[(self.path,self.owner,self.target)])

    def test_old_three_parent_include_is_not_omitted_without_edit_record(self):
        self.write('/../../../'+self.target)
        errors, checked = check_external_includes(self.root,self.catalog)
        self.assertEqual(len(errors),1)
        self.assertIn('missing current owner-relative include',errors[0])
        self.assertEqual(len(checked),1)

    def test_matching_test_folder_name_cannot_override_catalog_owner(self):
        self.write('/../../../../'+self.target)
        self.catalog['external_tests']['src/foundation/not-a-category'] = {'files':[{'path':self.path}]}
        errors,_ = check_external_includes(self.root,self.catalog)
        self.assertIn('ambiguous external owner',errors[0])

    def test_missing_target_rejected_even_with_correct_depth(self):
        self.write('/../../../../'+self.target)
        (self.root/self.target).unlink()
        errors,_ = check_external_includes(self.root,self.catalog)
        self.assertIn('missing current owner-relative include',errors[0])

    def test_retired_owner_is_not_reported_as_active_test_coverage(self):
        self.write('/../../../'+self.target)
        self.catalog['packages'] = {}
        self.assertEqual(check_external_includes(self.root,self.catalog),([],[]))

    def test_examples_in_comments_and_raw_strings_are_not_compiler_includes(self):
        example=b'include!(concat!(env!("CARGO_MANIFEST_DIR"), "/bad"))'
        data=b'// '+example+b'\n/* '+example+b' */\nlet text=r#"'+example+b'"#;\n'+example+b';'
        self.assertEqual([x[2] for x in manifest_includes(data)],['/bad'])

    def test_all_retained_current_owner_compiler_includes_resolve(self):
        catalog=json.loads((ROOT/'compliance/rust-sources.json').read_text())
        errors,checked=check_external_includes(ROOT,catalog)
        self.assertGreater(len(checked),100,'real retained corpus was not inspected')
        self.assertEqual(errors,[], '\n'.join(errors))

if __name__=='__main__':unittest.main(verbosity=2)
