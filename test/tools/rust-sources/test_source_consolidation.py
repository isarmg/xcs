"""Explicit multi-origin source consolidation retains every source and origin."""
import base64
from copy import deepcopy
import hashlib
from pathlib import Path
import sys
import tempfile
import unittest

TOOLS = Path(__file__).resolve().parents[3] / 'tools/rust-sources'
sys.path.insert(0, str(TOOLS))
import source_consolidation as consolidation


def digest(data):
    return hashlib.sha256(data).hexdigest()


def inventory(directory):
    if directory.is_symlink():
        raise ValueError('source symlink')
    if not directory.is_dir():
        raise ValueError('missing source root')
    result = {}
    for path in directory.rglob('*'):
        if path.is_symlink():
            raise ValueError('source symlink')
        if path.is_file():
            result[path.relative_to(directory).as_posix()] = digest(path.read_bytes())
        elif not path.is_dir():
            raise ValueError('source special file')
    return result


class SourceConsolidation(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.old = ['src/foundation/digest-1.0.0', 'src/foundation/block-buffer-2.0.0',
                    'src/foundation/crypto-common-3.0.0']
        self.destination = 'src/foundation/crypto-core'
        path = self.root / self.destination
        path.mkdir(parents=True)
        (path / 'Cargo.toml').write_text('[package]\nname = "crypto-core"\nversion = "1.0.0"\n')
        (path / 'src').mkdir()
        (path / 'src/lib.rs').write_text('mod buffer; mod common;\npub fn digest() {}\n')
        (path / 'src/buffer.rs').write_text('pub fn buffer() {}\n')
        (path / 'src/common.rs').write_text('pub fn common() {}\n')
        (path / 'src/facade.rs').write_text('pub const CONSOLIDATED: bool = true;\n')
        self.manifests, packages = {}, {}
        for index, directory in enumerate(self.old):
            name, version = [('digest', '1.0.0'), ('block-buffer', '2.0.0'), ('crypto-common', '3.0.0')][index]
            raw = ('[package]\nname = "' + name + '"\nversion = "' + version + '"\n').encode()
            self.manifests[directory] = raw
            packages[directory] = dict(directory=directory, name=name, version=version,
                origin_directory='vendor/' + directory.rsplit('/', 1)[1],
                upstream=dict(name=name, version=version, archive_sha256=digest(name.encode())),
                license='MIT', license_materials=['compliance/licenses/' + name + '/LICENSE'],
                repository='https://example.invalid/' + name,
                files_sha256={'Cargo.toml': digest(raw), 'src/lib.rs': digest(('fn ' + name + '() {}').encode())})
        self.previous = dict(schema=6, packages=packages, merges=[], external_tests={},
                             retired={}, superseded={}, renames={}, source_moves={})
        self.item = dict(primary_directory=self.old[0], destination_directory=self.destination,
            destination_name='crypto-core', reason='Combine the three cryptographic layers',
            inputs=[dict(package=deepcopy(packages[directory]), external_tests=None) for directory in self.old],
            destination_files_sha256=inventory(path),
            file_routes={directory: {'Cargo.toml': ({'destination': 'Cargo.toml'} if index == 0 else
                {'archive_base64': base64.b64encode(self.manifests[directory]).decode()}),
                'src/lib.rs': {'destination': ['src/lib.rs', 'src/buffer.rs', 'src/common.rs'][index]}}
                for index, directory in enumerate(self.old)},
            new_files={'src/facade.rs': digest((path / 'src/facade.rs').read_bytes())},
            destination_external_tests=None, external_file_routes={})
        self.plan = dict(schema=1, packages=[self.item])

    def apply(self, validator=None):
        return consolidation.prepare(self.root, self.previous, self.plan, inventory, validate_external=validator)

    def catalog(self, result=None):
        active, merges, records, changed = result or self.apply()
        return dict(self.previous, schema=8, packages=active, merges=merges,
                    consolidations=records, consolidation_records_sha256=consolidation.hashes(records))

    def reject(self, pattern='.'):
        with self.assertRaisesRegex(ValueError, pattern):
            self.apply()

    def test_three_to_one_preserves_exact_snapshots_and_all_origins(self):
        previous, plan = deepcopy(self.previous), deepcopy(self.plan)
        active, merges, records, changed = self.apply()
        self.assertEqual(self.previous, previous)
        self.assertEqual(self.plan, plan)
        self.assertEqual(set(active), {self.destination})
        self.assertEqual(active[self.destination]['origin_directory'], previous['packages'][self.old[0]]['origin_directory'])
        self.assertEqual([row['origin_directory'] for row in merges], [previous['packages'][p]['origin_directory'] for p in self.old[1:]])
        self.assertEqual(len(records), 1)
        self.assertEqual(list(records), [consolidation.receipt_id(self.item)])
        self.assertEqual(list(records.values()), [self.item])
        for package in [active[self.destination], *merges]:
            self.assertEqual(package['files_sha256'], self.item['destination_files_sha256'])
        self.assertEqual(set(changed), {self.destination})
        self.assertEqual(consolidation.validate_history(self.root, self.catalog()), records)

    def test_no_plan_never_learns_new_destination(self):
        active, merges, records, changed = consolidation.prepare(self.root, self.previous, None, inventory)
        self.assertEqual(active, self.previous['packages'])
        self.assertEqual((merges, records, changed), ([], {}, {}))

    def test_exact_previous_package_snapshot_required(self):
        self.item['inputs'][0]['package']['license'] = 'Apache-2.0'
        self.reject('exact previous')

    def test_exact_previous_external_snapshot_required(self):
        self.item['inputs'][0]['external_tests'] = {}
        self.reject('exact previous')

    def test_all_old_files_require_routes(self):
        del self.item['file_routes'][self.old[1]]['src/lib.rs']
        self.reject('every before file')

    def test_extra_old_routes_rejected(self):
        self.item['file_routes'][self.old[1]]['extra.rs'] = {'destination': 'src/lib.rs'}
        self.reject('every before file')

    def test_all_old_directories_require_routes(self):
        del self.item['file_routes'][self.old[1]]
        self.reject('exact input directories')

    def test_new_file_hash_must_match_destination(self):
        self.item['new_files']['src/facade.rs'] = '0' * 64
        self.reject('mapped/new coverage')

    def test_new_and_mapped_files_are_disjoint(self):
        self.item['new_files']['src/lib.rs'] = self.item['destination_files_sha256']['src/lib.rs']
        self.reject('mapped/new coverage')

    def test_all_destination_files_require_routes_or_new_receipts(self):
        self.item['new_files'] = {}
        self.reject('mapped/new coverage')

    def test_route_target_must_exist(self):
        self.item['file_routes'][self.old[1]]['src/lib.rs']['destination'] = 'src/missing.rs'
        self.reject('target missing')

    def test_many_to_one_reviewed_refactor_allowed(self):
        self.item['file_routes'][self.old[2]]['src/lib.rs'] = {'destination': 'src/buffer.rs'}
        self.item['new_files']['src/common.rs'] = self.item['destination_files_sha256']['src/common.rs']
        self.apply()

    def test_generic_discard_forbidden(self):
        self.item['file_routes'][self.old[1]]['src/lib.rs'] = {'disposition': 'reviewed removal'}
        self.reject('generic production discard')

    def test_source_archive_forbidden(self):
        self.item['file_routes'][self.old[1]]['src/lib.rs'] = {'archive_base64': base64.b64encode(b'fn block-buffer() {}').decode()}
        self.reject('only redundant')

    def test_primary_manifest_archive_forbidden(self):
        self.item['file_routes'][self.old[0]]['Cargo.toml'] = {'archive_base64': base64.b64encode(self.manifests[self.old[0]]).decode()}
        self.reject('only redundant')

    def test_archive_bytes_must_match_before_hash(self):
        self.item['file_routes'][self.old[1]]['Cargo.toml']['archive_base64'] = base64.b64encode(b'wrong').decode()
        self.reject('exact before hash')

    def test_archive_base64_must_be_valid(self):
        self.item['file_routes'][self.old[1]]['Cargo.toml']['archive_base64'] = '***'
        self.reject('manifest base64')

    def test_archive_size_is_bounded(self):
        self.item['file_routes'][self.old[1]]['Cargo.toml']['archive_base64'] = 'a' * (4 * ((consolidation.MAX_ARCHIVE_BYTES + 2) // 3) + 1)
        self.reject('size bound')

    def test_receipt_content_is_bounded(self):
        self.item['reason'] = 'a' * 10001
        self.reject('identity or reason')

    def test_input_count_and_schema_are_strict(self):
        for mutate in [lambda: self.plan.update(schema=True), lambda: self.item.update(inputs=self.item['inputs'][:1]),
                       lambda: self.plan.update(extra=True), lambda: self.item.update(extra=True)]:
            with self.subTest(mutate=mutate):
                before = deepcopy(self.plan)
                mutate()
                self.reject()
                self.plan = before
                self.item = self.plan['packages'][0]

    def test_duplicate_input_identity_rejected(self):
        self.item['inputs'].append(deepcopy(self.item['inputs'][0]))
        self.reject('duplicate')

    def test_duplicate_input_origin_rejected(self):
        self.item['inputs'][1]['package']['origin_directory'] = self.item['inputs'][0]['package']['origin_directory']
        self.reject('duplicate')

    def test_duplicate_plan_group_rejected(self):
        self.plan['packages'].append(deepcopy(self.item))
        self.reject('overlapping')

    def test_old_root_must_be_absent(self):
        (self.root / self.old[1]).mkdir()
        self.reject('old consolidated source root')

    def test_old_test_root_must_be_absent(self):
        (self.root / ('test/foundation/' + self.old[1].rsplit('/', 1)[1])).mkdir(parents=True)
        self.reject('old consolidated external test root')

    def test_missing_destination_root(self):
        self.item['destination_directory'] = 'src/foundation/missing'
        self.item['destination_name'] = 'missing'
        self.reject('present destination')

    def test_unreviewed_content_edits_rejected(self):
        (self.root / self.destination / 'src/buffer.rs').write_text('unreviewed')
        self.reject('content differs')

    def test_unreviewed_new_source_rejected(self):
        (self.root / self.destination / 'unreviewed.rs').write_text('unreviewed')
        self.reject('content differs')

    def test_manifest_name_and_version_preserved(self):
        path = self.root / self.destination / 'Cargo.toml'
        original = path.read_text()
        for value in [original.replace('1.0.0', '9.0.0'), original.replace('crypto-core', 'other')]:
            path.write_text(value)
            self.item['destination_files_sha256'] = inventory(path.parent)
            self.reject('manifest name/version')

    def test_workspace_manifest_version_is_supported(self):
        path = self.root / self.destination / 'Cargo.toml'
        path.write_text('[package]\nname = "crypto-core"\nversion.workspace = true\n')
        (self.root / 'Cargo.toml').write_text('[workspace.package]\nversion = "1.0.0"\n')
        self.item['destination_files_sha256'] = inventory(path.parent)
        self.apply()

    def test_destination_symlink_rejected(self):
        path = self.root / self.destination / 'link.rs'
        path.symlink_to('/tmp')
        self.reject('symlink')

    def test_unsafe_paths_rejected(self):
        for value in ['../escape', '/tmp/escape', 'src/foundation/../escape', 'src/foundation/a/b',
                      'src/foundation/a\\b', 'src/foundation/a:b', 'src/foundation/a\n']:
            with self.subTest(value=value):
                self.item['destination_directory'] = value
                self.reject()

    def test_unsafe_relative_routes_rejected(self):
        self.item['file_routes'][self.old[1]]['src/lib.rs']['destination'] = '../lib.rs'
        self.reject('unsafe')

    def test_root_symlink_rejected(self):
        alias = self.root.parent / (self.root.name + '-alias')
        alias.symlink_to(self.root)
        self.addCleanup(alias.unlink)
        with self.assertRaisesRegex(ValueError, 'root symlink'):
            consolidation.prepare(alias, self.previous, self.plan, inventory)

    def test_previously_merged_input_is_rejected(self):
        merged = deepcopy(self.previous['packages'][self.old[0]])
        merged['origin_directory'] = 'vendor/other'
        self.previous['merges'].append(merged)
        self.reject('already merged')

    def test_previously_renamed_input_is_rejected(self):
        self.previous['renames'] = {'old': {'destination_directory': self.old[0], 'package': {'directory': 'old'}}}
        self.reject('history|receipt')

    def test_previously_upgraded_input_is_rejected(self):
        self.previous['superseded'] = {'vendor/earlier': {'package': {'directory': self.old[0], 'origin_directory': 'vendor/earlier'}}}
        self.reject('upgraded inputs')

    def test_previously_moved_input_is_rejected(self):
        self.previous['source_moves'] = {'hash': {'directory': self.old[0]}}
        self.reject('history|receipt')

    def test_pinned_input_is_rejected(self):
        path = self.root / 'compliance/platform-test-relocation.json'
        path.parent.mkdir()
        path.write_text('{"directory": "' + self.old[0] + '"}')
        self.reject('pinned source')

    def test_history_is_content_addressed_and_tamper_evident(self):
        catalog = self.catalog()
        record = next(iter(catalog['consolidations'].values()))
        record['reason'] = 'edited'
        with self.assertRaisesRegex(ValueError, 'hash drift'):
            consolidation.validate_history(self.root, catalog)
        catalog['consolidation_records_sha256'] = consolidation.hashes(catalog['consolidations'])
        with self.assertRaisesRegex(ValueError, 'content-addressed'):
            consolidation.validate_history(self.root, catalog)

    def test_history_schema_gate(self):
        catalog = self.catalog()
        catalog['schema'] = 7
        with self.assertRaisesRegex(ValueError, 'schema 8'):
            consolidation.validate_history(self.root, catalog)

    def test_history_allows_later_reviewed_content_edits(self):
        catalog = self.catalog()
        history = deepcopy(catalog['consolidations'])
        path = self.root / self.destination / 'src/buffer.rs'
        path.write_text('later reviewed edit')
        current = inventory(path.parent.parent)
        for package in [*catalog['packages'].values(), *catalog['merges']]:
            package['files_sha256'] = current
        self.assertEqual(consolidation.validate_history(self.root, catalog), history)
        active, merges, retained, changed = consolidation.prepare(self.root, catalog, None, inventory)
        self.assertEqual(retained, history)
        self.assertEqual(changed, {})

    def test_history_rejects_lost_merge(self):
        catalog = self.catalog()
        catalog['merges'].pop()
        with self.assertRaisesRegex(ValueError, 'origin coverage'):
            consolidation.validate_history(self.root, catalog)

    def test_history_rejects_duplicate_origin(self):
        catalog = self.catalog()
        catalog['merges'].append(deepcopy(catalog['merges'][0]))
        with self.assertRaisesRegex(ValueError, 'duplicate current'):
            consolidation.validate_history(self.root, catalog)

    def test_history_rejects_primary_origin_substitution(self):
        catalog = self.catalog()
        catalog['packages'][self.destination]['origin_directory'] = 'vendor/replacement'
        with self.assertRaisesRegex(ValueError, 'same-origin active primary'):
            consolidation.validate_history(self.root, catalog)

    def test_history_rejects_legal_upstream_or_version_changes(self):
        for key, value in [('license', 'other'), ('license_materials', []), ('upstream', {'name': 'other'}), ('version', '9')]:
            with self.subTest(key=key):
                catalog = self.catalog()
                catalog['merges'][0][key] = value
                with self.assertRaisesRegex(ValueError, 'origin coverage'):
                    consolidation.validate_history(self.root, catalog)

    def test_history_rejects_retiring_consolidated_primary(self):
        catalog = self.catalog()
        catalog['retired'][self.destination] = catalog['packages'].pop(self.destination)
        with self.assertRaisesRegex(ValueError, 'same-origin active primary'):
            consolidation.validate_history(self.root, catalog)

    def test_history_rejects_secondary_as_separate_active_package(self):
        catalog = self.catalog()
        package = catalog['merges'].pop()
        catalog['packages'][self.old[2]] = package
        with self.assertRaisesRegex(ValueError, 'merged successor'):
            consolidation.validate_history(self.root, catalog)

    def test_history_current_merge_inventory_must_be_shared(self):
        catalog = self.catalog()
        catalog['merges'][0]['files_sha256']['src/buffer.rs'] = 'a' * 64
        with self.assertRaisesRegex(ValueError, 'shared inventory'):
            consolidation.validate_history(self.root, catalog)

    def test_history_returned_old_root_rejected(self):
        catalog = self.catalog()
        (self.root / self.old[0]).mkdir()
        with self.assertRaisesRegex(ValueError, 'old consolidated source root'):
            consolidation.validate_history(self.root, catalog)

    def test_stay_at_primary_directory_supported(self):
        original, destination = self.destination, 'src/foundation/digest-core'
        (self.root / original).rename(self.root / destination)
        old_primary = self.old[0]
        package = self.previous['packages'].pop(old_primary)
        package['directory'] = destination
        self.previous['packages'][destination] = package
        self.item['inputs'][0]['package'] = deepcopy(package)
        self.item.update(primary_directory=destination, destination_directory=destination, destination_name='digest-core')
        self.item['file_routes'][destination] = self.item['file_routes'].pop(old_primary)
        manifest = self.root / destination / 'Cargo.toml'
        manifest.write_text(manifest.read_text().replace('crypto-core', 'digest-core'))
        self.item['destination_files_sha256'] = inventory(manifest.parent)
        active, merges, records, changed = self.apply()
        self.assertEqual(set(active), {destination})
        self.assertEqual(len(merges), 2)

    def test_missing_external_validator_rejected(self):
        self.previous['external_tests'][self.old[0]] = {'opaque': True}
        self.item['inputs'][0]['external_tests'] = {'opaque': True}
        self.item['destination_external_tests'] = {'opaque': True}
        self.reject('explicit validator')

    def test_external_validator_checks_current_once_and_history_without_bytes(self):
        calls = []
        self.previous['external_tests'][self.old[0]] = {'opaque': True}
        self.item['inputs'][0]['external_tests'] = {'opaque': True}
        self.item['destination_external_tests'] = {'opaque': True}
        def validate(root, record, check_current=True):
            calls.append(check_current)
            self.assertEqual(root, self.root)
            self.assertEqual(record, self.item)
        result = self.apply(validate)
        self.assertEqual(calls, [True, False])
        catalog = self.catalog(result)
        consolidation.validate_history(self.root, catalog, validate_external=validate)
        self.assertEqual(calls, [True, False, False])

    def test_external_validator_failure_is_not_bypassed(self):
        def fail(*args, **kwargs):
            raise ValueError('transfer denied')
        with self.assertRaisesRegex(ValueError, 'transfer denied'):
            self.apply(fail)

    def test_normalized_previous_packages_exact(self):
        active, _, _, changed = self.apply()
        normalized = consolidation.remap_previous_packages(self.previous['packages'], changed)
        self.assertEqual(normalized, active)
        corrupted = deepcopy(self.previous['packages'])
        corrupted[self.old[0]]['name'] = 'other'
        with self.assertRaisesRegex(ValueError, 'does not match receipt'):
            consolidation.remap_previous_packages(corrupted, changed)

    def test_normalized_previous_external_exact_and_fresh_only(self):
        before = {'source': 'opaque'}
        after = {'destination': 'opaque'}
        self.previous['external_tests'][self.old[0]] = before
        self.item['inputs'][0]['external_tests'] = before
        self.item['destination_external_tests'] = after
        _, _, _, changed = self.apply(lambda *args, **kwargs: None)
        normalized = consolidation.remap_previous_external(self.previous['external_tests'], changed)
        self.assertEqual(normalized, {self.destination: after})
        with self.assertRaisesRegex(ValueError, 'does not match receipt'):
            consolidation.remap_previous_external(normalized, changed)
        consolidation.check_current_external(normalized, changed)
        with self.assertRaisesRegex(ValueError, 'differs from reviewed'):
            consolidation.check_current_external({}, changed)

    def additional_group(self):
        """Add a separate two-input CLI consolidation to the same repository."""
        destination = 'src/foundation/cli-core'
        current = self.root / destination
        (current / 'src').mkdir(parents=True)
        (current / 'Cargo.toml').write_text('[package]\nname = "cli-core"\nversion = "4.0.0"\n')
        (current / 'src/lib.rs').write_text('pub fn cli() {}\n')
        inputs, routes = [], {}
        for index, name in enumerate(('cli-builder', 'cli-lexer')):
            directory = 'src/foundation/' + name
            manifest = ('[package]\nname = "' + name + '"\nversion = "4.0.0"\n').encode()
            package = dict(directory=directory, name=name, version='4.0.0',
                origin_directory='vendor/' + name, upstream=dict(name=name, version='4.0.0'),
                files_sha256={'Cargo.toml': digest(manifest), 'src/lib.rs': digest(b'old')})
            self.previous['packages'][directory] = deepcopy(package)
            inputs.append(dict(package=package, external_tests=None))
            routes[directory] = {'Cargo.toml': {'destination': 'Cargo.toml'} if index == 0 else
                {'archive_base64': base64.b64encode(manifest).decode()},
                'src/lib.rs': {'destination': 'src/lib.rs'}}
        return dict(primary_directory=inputs[0]['package']['directory'], destination_directory=destination,
            destination_name='cli-core', reason='Merge parser and tokenization', inputs=inputs,
            destination_files_sha256=inventory(current), file_routes=routes, new_files={},
            destination_external_tests=None, external_file_routes={})

    def test_two_disjoint_groups_support_four_to_two(self):
        # Turn the original fixture into a two-input group with reviewed new glue.
        unused = self.item['inputs'].pop()['package']['directory']
        del self.previous['packages'][unused]
        del self.item['file_routes'][unused]
        self.item['new_files']['src/common.rs'] = self.item['destination_files_sha256']['src/common.rs']
        self.plan['packages'].append(self.additional_group())
        self.assertEqual(len(self.previous['packages']), 4)
        active, merges, history, changed = self.apply()
        self.assertEqual(len(active), 2)
        self.assertEqual(len(merges), 2)
        self.assertEqual(len(history), 2)
        self.assertEqual(len(changed), 2)

    def test_append_preserves_exact_prior_history(self):
        self.previous = self.catalog()
        prior = deepcopy(self.previous['consolidations'])
        self.item = self.additional_group()
        self.plan = dict(schema=1, packages=[self.item])
        active, merges, history, changed = self.apply()
        self.assertEqual(len(active), 2)
        self.assertEqual(len(merges), 3)
        self.assertEqual(len(history), 2)
        self.assertEqual({key: history[key] for key in prior}, prior)
        self.assertEqual(set(changed), {'src/foundation/cli-core'})

    def test_no_plan_still_authenticates_old_history(self):
        catalog = self.catalog()
        next(iter(catalog['consolidations'].values()))['reason'] = 'tampered'
        with self.assertRaisesRegex(ValueError, 'hash drift'):
            consolidation.prepare(self.root, catalog, None, inventory)

    def test_prior_consolidated_destination_cannot_be_reused(self):
        self.previous = self.catalog()
        prior = self.additional_group()
        prior['inputs'][0]['package'] = deepcopy(self.previous['packages'][self.destination])
        old_primary = prior['primary_directory']
        prior['primary_directory'] = self.destination
        prior['file_routes'][self.destination] = {
            path: {'destination': 'Cargo.toml' if path == 'Cargo.toml' else 'src/lib.rs'}
            for path in prior['inputs'][0]['package']['files_sha256']}
        del prior['file_routes'][old_primary]
        self.item, self.plan = prior, dict(schema=1, packages=[prior])
        self.reject('already merged|chain-aware')

    def test_unrelated_existing_merges_preserved(self):
        unrelated = deepcopy(self.previous['packages'][self.old[1]])
        unrelated.update(directory='src/interface', origin_directory='vendor/unrelated')
        self.previous['merges'].append(unrelated)
        _, merges, _, _ = self.apply()
        self.assertEqual(merges[0], unrelated)

    def test_hash_mapping_rejects_non_string_keys(self):
        with self.assertRaisesRegex(ValueError, 'hash mapping'):
            consolidation.hashes({1: {}, 'a': {}})

    def test_validated_origin_cross_check(self):
        _, _, records, _ = self.apply()
        origins = {p['origin_directory']: dict(upstream=p['upstream'], license=p['license'],
                   central_license_materials=p['license_materials']) for p in self.previous['packages'].values()}
        consolidation.validate_origins(records, origins)
        next(iter(origins.values()))['upstream'] = {'name': 'other'}
        with self.assertRaisesRegex(ValueError, 'historical upstream'):
            consolidation.validate_origins(records, origins)


if __name__ == '__main__':
    unittest.main()
