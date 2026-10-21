"""Manager-level multi-origin consolidation and post-transfer invariants."""
import base64
from contextlib import redirect_stderr, redirect_stdout
from copy import deepcopy
import io
import json
from pathlib import Path
import shutil
import sys
import unittest
from unittest.mock import patch

import test_manage as fixture

manager = fixture.manager


class ManageConsolidation(unittest.TestCase):
    # Reuse the fixture methods, not the base class's unrelated test methods.
    origin = fixture.SourcePolicy.origin
    write_json = fixture.SourcePolicy.write_json
    save_catalog = fixture.SourcePolicy.save_catalog
    refresh = fixture.SourcePolicy.refresh
    verify = fixture.SourcePolicy.verify

    def setUp(self):
        fixture.SourcePolicy.setUp(self)
        self.primary = 'src/foundation/mini'
        self.secondary = 'src/foundation/side'
        self.destination = 'src/foundation/combined-core'
        secondary = self.root / self.secondary
        secondary.mkdir()
        (secondary / 'Cargo.toml').write_text('[package]\nname = "side"\nversion = "2.0.0"\n')
        (secondary / 'lib.rs').write_text('pub fn value() -> u8 { 2 }\n')
        package = deepcopy(self.catalog['packages'][self.primary])
        package.update(name='side', version='2.0.0', directory=self.secondary,
                       origin_directory='old/normalization', upstream=self.merged_upstream,
                       files_sha256=manager.files(secondary))
        self.catalog['packages'][self.secondary] = package
        self.catalog['merges'] = []
        shutil.rmtree(self.root / 'src/merged')
        self._workspace([(self.primary, 'mini', '1.0.0'), (self.secondary, 'side', '2.0.0')])
        self.save_catalog()
        self.refresh()
        self.verify()

    def _workspace(self, sources):
        members = json.dumps([directory for directory, _, _ in sources])
        (self.root / 'Cargo.toml').write_text('[workspace]\nmembers = ["src/app"]\nexclude = ' + members + '\n')
        app = '[package]\nname = "app"\nversion = "1.0.0"\n[dependencies]\n'
        lock = 'version = 4\n[[package]]\nname = "app"\nversion = "1.0.0"\n'
        packages = [dict(id='app', name='app', manifest_path=str(self.root / 'src/app/Cargo.toml'), source=None, targets=[{'kind': ['lib']}])]
        deps, nodes = [], []
        for directory, name, version in sources:
            app += name + ' = { path = "../foundation/' + directory.rsplit('/', 1)[1] + '" }\n'
            lock += '[[package]]\nname = "' + name + '"\nversion = "' + version + '"\n'
            packages.append(dict(id=name, name=name, manifest_path=str(self.root / directory / 'Cargo.toml'), source=None, targets=[{'kind': ['lib']}]))
            deps.append(dict(pkg=name, dep_kinds=[{'kind': None}]))
            nodes.append(dict(id=name, features=[], deps=[]))
        (self.root / 'src/app/Cargo.toml').write_text(app)
        (self.root / 'Cargo.lock').write_text(lock)
        self.metadata = dict(packages=packages, workspace_members=['app'], resolve=dict(nodes=[dict(id='app', features=[], deps=deps), *nodes]))

    def _install_external_baseline(self):
        owners = []
        for directory, label in ((self.primary, 'mini'), (self.secondary, 'side')):
            relative = 'test/foundation/' + label
            path = self.root / relative
            path.mkdir(parents=True)
            data = ('assert_eq!(' + label + '::value(), ' + ('1' if label == 'mini' else '2') + ');\n').encode()
            (path / 'tests.rs').write_bytes(data)
            owners.append(dict(directory=directory, roots=[relative], files=[dict(
                source=directory + '/lib.rs', path=relative + '/tests.rs',
                source_sha256_before=self.catalog['packages'][directory]['files_sha256']['lib.rs'], sha256=manager.digest(data))]))
        self.mapping = self.root / 'compliance/external-tests/consolidation-fixture.json'
        self.mapping.parent.mkdir(parents=True)
        self.mapping.write_text(json.dumps(dict(schema_version=1, owners=owners)))
        current, receipts = manager.external_tests.load(self.root, self.catalog['packages'])
        self.catalog.update(external_tests=current, external_test_mappings_sha256=receipts)
        self.save_catalog()
        # The fixture starts with an authenticated synthetic legacy test receipt.
        # Do not substitute the production corpus's unrelated pinned bootstrap.
        stub = patch.object(manager.source_relocation, 'read_receipt', return_value={})
        stub.start()
        self.addCleanup(stub.stop)
        self.verify()

    def stage(self, external=False):
        if external:
            self._install_external_baseline()
        previous = deepcopy(self.catalog)
        destination = self.root / self.destination
        destination.mkdir()
        (destination / 'Cargo.toml').write_text('[package]\nname = "combined-core"\nversion = "1.0.0"\n')
        (destination / 'lib.rs').write_text('pub mod side;\npub fn value() -> u8 { 1 }\n')
        (destination / 'side.rs').write_bytes((self.root / self.secondary / 'lib.rs').read_bytes())
        (destination / 'facade.rs').write_text('pub const INTEGRATED: bool = true;\n')
        side_manifest = (self.root / self.secondary / 'Cargo.toml').read_bytes()
        record = dict(primary_directory=self.primary, destination_directory=self.destination,
            destination_name='combined-core', reason='Integrate the value and side implementations',
            inputs=[dict(package=deepcopy(previous['packages'][directory]),
                         external_tests=deepcopy(previous.get('external_tests', {}).get(directory)))
                    for directory in (self.primary, self.secondary)],
            destination_files_sha256=manager.files(destination),
            file_routes={self.primary: {'Cargo.toml': {'destination': 'Cargo.toml'}, 'lib.rs': {'destination': 'lib.rs'}},
                         self.secondary: {'Cargo.toml': {'archive_base64': base64.b64encode(side_manifest).decode()},
                                          'lib.rs': {'destination': 'side.rs'}}},
            new_files={'facade.rs': manager.digest((destination / 'facade.rs').read_bytes())},
            destination_external_tests=None, external_file_routes={})
        if external:
            root = 'test/foundation/combined-core'
            (self.root / root).mkdir()
            after = dict(directory=self.destination, origin_directory=previous['packages'][self.primary]['origin_directory'],
                         roots=[root], files=[], file_count=2, bytes=0)
            for item, label in zip(record['inputs'], ('mini', 'side')):
                before = item['external_tests']
                row = before['files'][0]
                data = (self.root / row['path']).read_bytes()
                old, new = (label + '::').encode(), ('combined_core::' + ('side::' if label == 'side' else '')).encode()
                changed = data.replace(old, new)
                path = root + '/' + label + '.rs'
                (self.root / path).write_bytes(changed)
                after['files'].append(dict(row, path=path, sha256=manager.digest(changed), origin_directory=before['origin_directory']))
                after['bytes'] += len(changed)
                record['external_file_routes'][row['path']] = dict(destination=path,
                    replacements=[dict(old=base64.b64encode(old).decode(), new=base64.b64encode(new).decode())])
                shutil.rmtree(self.root / before['roots'][0])
            record['destination_external_tests'] = after
            self._write_after_mapping(after)
        shutil.rmtree(self.root / self.primary)
        shutil.rmtree(self.root / self.secondary)
        self.source = destination
        self._workspace([(self.destination, 'combined-core', '1.0.0')])
        return dict(schema=1, packages=[record]), previous

    def _write_after_mapping(self, owner):
        mapping_owner = {key: value for key, value in owner.items() if key not in ('origin_directory', 'file_count', 'bytes')}
        self.mapping.write_text(json.dumps(dict(schema_version=1, owners=[mapping_owner])))

    def commit(self, external=False):
        plan, previous = self.stage(external=external)
        self.refresh(consolidate_plan_data=plan)
        self.verify()
        return plan, previous

    def assert_failed_without_catalog_write(self, action, message):
        before = (self.root / manager.CATALOG).read_bytes()
        with self.assertRaisesRegex(ValueError, message):
            action()
        self.assertEqual((self.root / manager.CATALOG).read_bytes(), before)

    def test_prepare_refresh_verify_repeat_keep_exact_history(self):
        origins = (self.root / manager.ORIGINS).read_bytes()
        plan, previous = self.stage()
        active, merges, history, changed = manager.source_consolidation.prepare(
            self.root, previous, plan, manager.files, validate_external=manager.external_transfers.validate)
        self.assertEqual(set(active), {self.destination})
        self.assertEqual(len(merges), 1)
        self.refresh(consolidate_plan_data=plan)
        self.verify()
        with patch.object(manager, 'cargo_metadata', return_value=self.metadata):
            self.verify(metadata=True)
        self.assertEqual(self.catalog['schema'], 8)
        self.assertEqual(self.catalog['consolidations'], history)
        self.assertEqual(self.catalog['packages'], active)
        self.assertEqual(self.catalog['merges'], merges)
        self.assertEqual(self.catalog['origin_records_sha256'], previous['origin_records_sha256'])
        self.assertEqual((self.root / manager.ORIGINS).read_bytes(), origins)
        first_catalog = (self.root / manager.CATALOG).read_bytes()
        first_document = (self.root / manager.DOC).read_bytes()
        self.refresh()
        self.verify()
        self.assertEqual((self.root / manager.CATALOG).read_bytes(), first_catalog)
        self.assertEqual((self.root / manager.DOC).read_bytes(), first_document)
        self.assertEqual(self.catalog['consolidations'], history)

    def test_unplanned_consolidation_rejected(self):
        self.stage()
        self.assert_failed_without_catalog_write(self.refresh, 'unregistered source directories')

    def test_later_reviewed_current_source_edit_preserves_history(self):
        self.commit()
        history = deepcopy(self.catalog['consolidations'])
        (self.source / 'side.rs').write_text('pub fn value() -> u8 { 3 }\n')
        with self.assertRaisesRegex(ValueError, 'source content drift'):
            self.verify()
        self.refresh()
        self.verify()
        self.assertEqual(self.catalog['consolidations'], history)
        self.assertEqual(self.catalog['packages'][self.destination]['files_sha256'], self.catalog['merges'][0]['files_sha256'])

    def test_merge_origin_loss_rejected_by_refresh_and_verify(self):
        self.commit()
        self.catalog['merges'] = []
        self.save_catalog()
        for action in (self.verify, self.refresh):
            self.assert_failed_without_catalog_write(action, 'origin coverage')

    def test_primary_origin_substitution_rejected(self):
        self.commit()
        self.catalog['packages'][self.destination]['origin_directory'] = 'old/unrecognized'
        self.save_catalog()
        for action in (self.verify, self.refresh):
            self.assert_failed_without_catalog_write(action, 'same-origin active primary')

    def test_immutable_history_tamper_rejected(self):
        self.commit()
        next(iter(self.catalog['consolidations'].values()))['reason'] = 'rewritten'
        self.save_catalog()
        for action in (self.verify, self.refresh):
            self.assert_failed_without_catalog_write(action, 'hash drift')

    def test_post_consolidation_lifecycle_operations_require_adapter(self):
        self.commit()
        for option in ('retire_plan', 'upgrade_plan_data', 'rename_plan_data', 'move_plan_data'):
            with self.subTest(option=option):
                plan = dict(schema=1, packages=[dict(directory=self.destination)])
                self.assert_failed_without_catalog_write(lambda: self.refresh(**{option: plan}), 'chain-aware adapter')

    def test_initial_consolidation_cannot_overlap_other_lifecycle_plan(self):
        consolidation_plan, _ = self.stage()
        for option in ('retire_plan', 'upgrade_plan_data', 'rename_plan_data', 'move_plan_data'):
            for directory in (self.primary, self.destination):
                with self.subTest(option=option, directory=directory):
                    plan = dict(schema=1, packages=[dict(directory=directory)])
                    self.assert_failed_without_catalog_write(
                        lambda: self.refresh(consolidate_plan_data=consolidation_plan, **{option: plan}),
                        'consolidation and other lifecycle operations overlap')

    def test_two_origin_external_transfer_then_repeat_refresh(self):
        plan, previous = self.commit(external=True)
        history = deepcopy(self.catalog['consolidations'])
        owner = self.catalog['external_tests'][self.destination]
        self.assertEqual(owner, plan['packages'][0]['destination_external_tests'])
        self.assertEqual({row['origin_directory'] for row in owner['files']}, {'old/mini', 'old/normalization'})
        self.assertEqual({row['source'] for row in owner['files']}, {self.primary + '/lib.rs', self.secondary + '/lib.rs'})
        self.refresh()
        self.verify()
        self.assertEqual(self.catalog['consolidations'], history)
        self.assertEqual(self.catalog['external_tests'][self.destination], owner)
        self.assertEqual(self.catalog['origin_records_sha256'], previous['origin_records_sha256'])

    def test_explicit_later_external_edit_preserves_transfer_proof(self):
        self.commit(external=True)
        history = deepcopy(self.catalog['consolidations'])
        mapping = json.loads(self.mapping.read_text())
        row = mapping['owners'][0]['files'][1]
        (self.root / row['path']).write_text('assert_eq!(combined_core::side::value(), 99);\n')
        self.assert_failed_without_catalog_write(self.refresh, 'inventory/content drift')
        row['sha256'] = manager.digest((self.root / row['path']).read_bytes())
        self.mapping.write_text(json.dumps(mapping))
        self.refresh()
        self.verify()
        self.assertEqual(self.catalog['consolidations'], history)
        self.assertEqual(self.catalog['external_tests'][self.destination]['files'][1]['sha256'], row['sha256'])

    def test_removing_post_transfer_row_origin_rejected(self):
        self.commit(external=True)
        original = self.mapping.read_text()
        for index in (0, 1):
            with self.subTest(row=index):
                mapping = json.loads(original)
                mapping['owners'][0]['files'][index].pop('origin_directory')
                self.mapping.write_text(json.dumps(mapping))
                for action in (self.verify, self.refresh):
                    self.assert_failed_without_catalog_write(action, 'ownership|origin|accounting|attribution')
        self.mapping.write_text(original)

    def test_changing_post_transfer_row_origin_rejected(self):
        self.commit(external=True)
        mapping = json.loads(self.mapping.read_text())
        mapping['owners'][0]['files'][1]['origin_directory'] = 'old/mini'
        self.mapping.write_text(json.dumps(mapping))
        for action in (self.verify, self.refresh):
            self.assert_failed_without_catalog_write(action, 'ownership|origin|accounting|attribution')

    def test_fabricated_post_transfer_source_hash_or_origin_grants_rejected(self):
        self.commit(external=True)
        original = self.mapping.read_text()
        for field, value in [('source', self.primary + '/unrecorded.rs'),
                             ('source_sha256_before', 'f' * 64),
                             ('origin_directory', 'old/fabricated')]:
            with self.subTest(field=field):
                mapping = json.loads(original)
                mapping['owners'][0]['files'][0][field] = value
                self.mapping.write_text(json.dumps(mapping))
                for action in (self.verify, self.refresh):
                    self.assert_failed_without_catalog_write(action, 'ownership|origin|accounting|attribution')
        self.mapping.write_text(original)

    def test_returning_old_source_or_test_root_rejected(self):
        self.commit(external=True)
        for relative in (self.primary, 'test/foundation/mini'):
            with self.subTest(root=relative):
                path = self.root / relative
                path.mkdir()
                if relative == self.primary:
                    (path / 'Cargo.toml').write_text('[package]\nname = "mini"\nversion = "1.0.0"\n')
                for action in (self.verify, self.refresh):
                    self.assert_failed_without_catalog_write(action, 'old consolidated .* root')
                shutil.rmtree(path)

    def test_cli_consolidate_plan_runs_real_refresh(self):
        plan, _ = self.stage(external=True)
        path = self.root / 'consolidation-plan.json'
        path.write_text(json.dumps(plan))
        with patch.object(manager, 'ROOT', self.root), patch.object(sys, 'argv', ['manage.py', 'refresh', '--consolidate-plan', str(path)]), \
             patch.object(manager, 'cargo_metadata', return_value=self.metadata), redirect_stdout(io.StringIO()):
            self.assertEqual(manager.main(), 0)
        self.catalog = manager.read_json(self.root / manager.CATALOG)
        self.verify()
        self.assertEqual(self.catalog['schema'], 8)
        self.assertEqual(len(self.catalog['consolidations']), 1)

    def test_cli_verify_rejects_consolidate_plan_before_reading_it(self):
        missing = self.root / 'missing-plan.json'
        with patch.object(manager, 'ROOT', self.root), patch.object(sys, 'argv', ['manage.py', 'verify', '--consolidate-plan', str(missing)]), \
             redirect_stderr(io.StringIO()) as error, patch.object(manager, 'read_json') as read:
            self.assertEqual(manager.main(), 1)
        self.assertIn('only accepted by refresh', error.getvalue())
        read.assert_not_called()


if __name__ == '__main__':
    unittest.main()
