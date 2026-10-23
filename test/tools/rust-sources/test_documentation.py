"""Current dependency guides cover identities without rewriting source evidence."""
from copy import deepcopy
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
SPEC = importlib.util.spec_from_file_location('documentation', ROOT / 'tools/rust-sources/documentation.py')
documentation = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(documentation)


class DependencyDocumentation(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.catalog = {'packages': {}, 'merges': [], 'retired': {'not-current': {}}, 'superseded': {}}
        self.origins = {'packages': []}
        self.receipt = {'schema': 1, 'documents': []}
        self.write('docs/dependencies/README.md', '# Entry\n')
        self.write(documentation.ORIGINS, '{}')

    def write(self, path, contents):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(contents, encoding='utf-8')
        return target

    def source(self, name='example', *, directory=None, upstream=None, merged=False, readme=True, rustdoc=True):
        directory = directory or 'src/foundation/collections/' + name
        upstream = upstream or name
        identity = upstream + '-1.0.0'
        origin_key = 'vendor/rust/' + identity
        if not (self.root / directory / 'Cargo.toml').exists():
            self.write(directory + '/Cargo.toml', f'[package]\nname = "{name}"\nversion = "1.0.0"\n')
            self.write(directory + '/src/lib.rs', ('//! Current API documentation.\n' if rustdoc else '') + 'pub fn example() {}\n')
        license_path = 'compliance/licenses/upstream/' + identity + '/LICENSE-MIT'
        metadata = 'compliance/provenance/packages/' + identity + '/Cargo.toml.orig.txt'
        self.write(license_path, 'Original license\n')
        self.write(metadata, 'Original manifest\n')
        original = {'origin_directory': origin_key, 'identity': identity,
                    'upstream': {'name': upstream, 'version': '1.0.0',
                                 'archive_url': 'https://example.test/' + identity + '.crate',
                                 'archive_sha256': 'a' * 64},
                    'central_original_documentation': [], 'original_files_sha256': {},
                    'central_original_metadata': [metadata], 'central_license_materials': [license_path]}
        self.origins['packages'].append(original)
        row = {'name': name, 'version': '1.0.0', 'directory': directory,
               'origin_directory': origin_key, 'upstream': deepcopy(original['upstream']),
               'license_materials': [license_path], 'changes': 'History that must not be reprinted'}
        if merged:
            self.catalog['merges'].append(row)
        else:
            self.catalog['packages'][directory] = row
        if readme:
            self.document(original, 'README.md', '# Original usage\n')
        return row, original

    def document(self, origin, relative, content):
        path = 'docs/upstream/' + origin['identity'] + '/' + relative
        self.write(path, content)
        origin['central_original_documentation'].append(path)
        origin['original_files_sha256'][relative.replace('upstream-github/', '.github/', 1)] = hashlib.sha256(content.encode()).hexdigest()
        return path

    def supplement(self, origin, content='# Exact release usage\n', path=None):
        upstream = origin['upstream']
        path = path or f"docs/dependencies/originals/{upstream['name']}-{upstream['version']}/README.md"
        self.write(path, content)
        row = {'origin_directory': origin['origin_directory'], 'upstream_name': upstream['name'],
               'upstream_version': upstream['version'], 'archive_url': upstream['archive_url'],
               'archive_sha256': upstream['archive_sha256'], 'original_path': 'README.md',
               'path': path, 'sha256': hashlib.sha256(content.encode()).hexdigest(),
               'bytes': len(content.encode()), 'license_materials': origin['central_license_materials']}
        self.receipt['documents'].append(row)
        return row

    def render(self):
        return documentation.render(self.root, self.catalog, self.origins, self.receipt)

    def test_current_identity_coverage_groups_and_merged_owners(self):
        row, origin = self.source(name='renamed', upstream='original')
        self.source(name='former', upstream='absorbed', directory=row['directory'], merged=True)
        self.source(name='other', directory='src/foundation/networking/other')
        self.source(name='name-policy', directory='src/naming', merged=True)
        self.write('docs/components/naming.md', '# Current naming\n')
        self.catalog['retired']['unused'] = {'directory': 'src/foundation/removed/unused'}
        before = deepcopy((self.catalog, self.origins, self.receipt))
        pages = self.render()
        self.assertEqual(set(pages), {'docs/dependencies/collections.md', 'docs/dependencies/networking.md',
                                      'docs/dependencies/integrated.md'})
        collection = pages['docs/dependencies/collections.md']
        self.assertIn('## renamed', collection)
        self.assertIn('original@1.0.0', collection)
        self.assertIn('absorbed@1.0.0（合并来源）', collection)
        self.assertEqual(collection.count('## renamed'), 1)
        self.assertNotIn('History that must not be reprinted', ''.join(pages.values()))
        self.assertNotIn('unused', ''.join(pages.values()))
        self.assertEqual((self.catalog, self.origins, self.receipt), before)
        self.assertEqual(pages, self.render())

    def test_guide_selection_excludes_history_and_process(self):
        _, origin = self.source()
        for path in ('CHANGELOG.md', 'RELEASE_PROCESS.md', 'CONTRIBUTING.md', 'upstream-github/ISSUE_TEMPLATE/readme.md'):
            self.document(origin, path, '# Not a current guide\n')
        self.document(origin, 'DESIGN.md', '# Useful design\n')
        text = ''.join(self.render().values())
        self.assertIn('README.md', text)
        self.assertIn('DESIGN.md', text)
        self.assertNotIn('CHANGELOG', text)
        self.assertNotIn('RELEASE_PROCESS', text)
        self.assertNotIn('CONTRIBUTING', text)
        self.assertNotIn('ISSUE_TEMPLATE', text)

    def test_missing_excluded_original_is_not_silently_ignored(self):
        _, origin = self.source()
        path = self.document(origin, 'CHANGELOG.md', 'History\n')
        (self.root / path).unlink()
        with self.assertRaisesRegex(documentation.DocumentationError, 'Missing file'):
            self.render()

    def test_original_document_bytes_are_checked(self):
        _, origin = self.source()
        self.write(origin['central_original_documentation'][0], '# Changed\n')
        with self.assertRaisesRegex(documentation.DocumentationError, 'Original document bytes'):
            self.render()

    def test_missing_document_inventory_is_not_treated_as_no_docs(self):
        _, origin = self.source()
        del origin['central_original_documentation']
        with self.assertRaises((documentation.DocumentationError, KeyError)):
            self.render()

    def test_explicit_rustdoc_fallback_without_empty_placeholder(self):
        row, _ = self.source(readme=False)
        text = ''.join(self.render().values())
        self.assertIn('未附独立使用文档', text)
        self.assertIn('当前源码内 Rustdoc', text)
        self.assertIn(row['directory'] + '/src/lib.rs', text)
        self.assertFalse((self.root / 'docs/upstream').exists())

    def test_missing_rustdoc_is_an_error_not_an_invented_guide(self):
        self.source(readme=False, rustdoc=False)
        with self.assertRaisesRegex(documentation.DocumentationError, 'no current Rustdoc fallback'):
            self.render()

    def test_merged_fallback_uses_identity_specific_module(self):
        row, _ = self.source(name='url-encoding', directory='src/foundation/networking/url-encoding')
        self.write('docs/architecture/url-encoding.md', '# Current URL API\n')
        self.source(name='form_urlencoded', upstream='form_urlencoded', directory=row['directory'], merged=True, readme=False)
        self.write(row['directory'] + '/src/form/mod.rs', '//! Form encoding API\n')
        text = ''.join(self.render().values())
        self.assertIn('当前源码内 Rustdoc](../../' + row['directory'] + '/src/form/mod.rs)', text)

    def test_unknown_identity_is_rejected(self):
        row, _ = self.source()
        row['origin_directory'] = 'vendor/unknown'
        with self.assertRaisesRegex(documentation.DocumentationError, 'Unknown current identity'):
            self.render()

    def test_duplicate_current_identity_is_rejected(self):
        row, _ = self.source()
        self.catalog['merges'].append(deepcopy(row))
        with self.assertRaisesRegex(documentation.DocumentationError, 'Duplicate current identity'):
            self.render()

    def test_unknown_group_or_nonfoundation_owner_is_rejected(self):
        self.source(directory='src/foundation/unknown/example')
        with self.assertRaisesRegex(documentation.DocumentationError, 'Unknown foundation group'):
            self.render()
        self.assertRaises(documentation.DocumentationError, documentation.group_for, 'src/new-component')

    def test_current_and_original_identity_must_agree(self):
        row, _ = self.source()
        row['upstream']['version'] = '2.0.0'
        with self.assertRaisesRegex(documentation.DocumentationError, 'Upstream identity mismatch'):
            self.render()

    def test_exact_supplement_replaces_import_guide_but_preserves_original(self):
        _, origin = self.source()
        old = self.root / origin['central_original_documentation'][0]
        before = old.read_bytes()
        supplement = self.supplement(origin)
        text = ''.join(self.render().values())
        self.assertIn(supplement['path'], text)
        self.assertNotIn('../../' + origin['central_original_documentation'][0], text)
        self.assertIn('精确发布包原件', text)
        self.assertEqual(old.read_bytes(), before)

    def test_identical_registered_original_can_be_reused(self):
        _, existing = self.source(name='existing')
        _, origin = self.source(name='supplemented', readme=False)
        path = existing['central_original_documentation'][0]
        self.supplement(origin, content='# Original usage\n', path=path)
        text = ''.join(self.render().values())
        self.assertIn('supplemented@1.0.0', text)
        self.assertIn(path, text)
        self.assertFalse((self.root / documentation.ORIGINALS).exists())

    def test_supplement_tamper_missing_size_and_identity_are_rejected(self):
        _, origin = self.source(readme=False)
        entry = self.supplement(origin)
        path = self.root / entry['path']
        data = path.read_bytes()
        for field, bad in (('bytes', entry['bytes'] + 1), ('sha256', 'b' * 64),
                           ('upstream_version', '9.0'), ('archive_sha256', 'b' * 64)):
            with self.subTest(field=field):
                old = entry[field]
                entry[field] = bad
                self.assertRaises(documentation.DocumentationError, self.render)
                entry[field] = old
        path.write_bytes(b'Tampered')
        self.assertRaises(documentation.DocumentationError, self.render)
        path.write_bytes(data)
        path.unlink()
        self.assertRaises(documentation.DocumentationError, self.render)

    def test_supplement_paths_fields_duplicates_and_unregistered_files_are_rejected(self):
        _, origin = self.source(readme=False)
        entry = self.supplement(origin)
        for field, bad in (('path', '../escape.md'), ('original_path', '../README.md'),
                           ('original_path', '/README.md'), ('bytes', True)):
            with self.subTest(field=field, value=bad):
                old = entry[field]
                entry[field] = bad
                self.assertRaises(documentation.DocumentationError, self.render)
                entry[field] = old
        entry['unrecognized'] = True
        self.assertRaises(documentation.DocumentationError, self.render)
        del entry['unrecognized']
        self.receipt['documents'].append(deepcopy(entry))
        self.assertRaises(documentation.DocumentationError, self.render)
        self.receipt['documents'].pop()
        self.write(documentation.ORIGINALS + '/untracked.md', 'Untracked')
        with self.assertRaisesRegex(documentation.DocumentationError, 'inventory mismatch'):
            self.render()

    def test_symlink_input_and_ancestor_are_rejected(self):
        _, origin = self.source()
        target = self.root / origin['central_original_documentation'][0]
        target.unlink()
        target.symlink_to(self.root / 'docs/dependencies/README.md')
        self.assertRaises(documentation.DocumentationError, self.render)
        target.unlink()
        target.parent.rmdir()
        target.parent.symlink_to(self.root / 'docs/dependencies', target_is_directory=True)
        self.assertRaises(documentation.DocumentationError, self.render)

    def test_refresh_verify_drift_and_immutable_files(self):
        _, origin = self.source()
        pages = self.render()
        before = {path: path.read_bytes() for path in self.root.rglob('*') if path.is_file()}
        documentation.synchronize(self.root, pages, 'refresh')
        documentation.synchronize(self.root, pages, 'verify')
        for path, data in before.items():
            self.assertEqual(path.read_bytes(), data)
        self.write(next(iter(pages)), 'Stale')
        with self.assertRaisesRegex(documentation.DocumentationError, 'drift'):
            documentation.synchronize(self.root, pages, 'verify')
        documentation.synchronize(self.root, pages, 'refresh')
        documentation.synchronize(self.root, pages, 'verify')
        self.write('docs/dependencies/obsolete.md', 'Old owner')
        self.assertRaises(documentation.DocumentationError, documentation.synchronize, self.root, pages, 'refresh')

    def test_refresh_rejects_outside_output_and_symlink_targets(self):
        self.source()
        pages = self.render()
        for path in ('docs/dependencies/README.md', 'docs/upstream/README.md', '../escape.md'):
            self.assertRaises(documentation.DocumentationError, documentation.synchronize,
                              self.root, {path: 'Override'}, 'refresh')
        target = self.root / next(iter(pages))
        target.symlink_to(self.root / 'docs/dependencies/README.md')
        self.assertRaises(documentation.DocumentationError, documentation.synchronize, self.root, pages, 'refresh')
        target.unlink()
        (self.root / 'docs/dependencies/README.md').unlink()
        (self.root / 'docs/dependencies').rmdir()
        (self.root / 'docs/dependencies').symlink_to(self.root / 'docs/upstream', target_is_directory=True)
        self.assertRaises(documentation.DocumentationError, documentation.synchronize, self.root, pages, 'refresh')


class RepositoryDocumentation(unittest.TestCase):
    def test_current_repository_has_exactly_one_entry_per_active_or_merged_identity(self):
        catalog = json.loads((ROOT / documentation.CATALOG).read_text())
        origins = json.loads((ROOT / documentation.ORIGINS).read_text())
        receipt = json.loads((ROOT / documentation.SUPPLEMENT).read_text())
        pages = documentation.render(ROOT, catalog, origins, receipt)
        self.assertEqual(set(pages), {f'docs/dependencies/{group}.md' for group in (*documentation.GROUPS, 'integrated')})
        rows = [line for text in pages.values() for line in text.splitlines()
                if line.startswith('| ') and '@' in line.split('|')[1]]
        self.assertEqual(len(rows), len(catalog['packages']) + len(catalog['merges']))
        self.assertEqual(len(rows), len({line.split('|')[1].strip() for line in rows}))
        self.assertEqual(sum('未附独立' in line.split('|')[2] for line in rows), 18)


if __name__ == '__main__':
    unittest.main()
