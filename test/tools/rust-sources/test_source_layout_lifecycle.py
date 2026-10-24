"""Post-layout lifecycle regression coverage against the real source manager.

Never modifies the repository: each scenario uses SourcePolicy's isolated tree.
The grouping installation remains exact; later retirement must use its existing
explicit reviewed plan and preserve all old evidence and historical identities.
"""
from copy import deepcopy
import importlib
import json
from pathlib import Path
import shutil
import sys
import unittest

sys.path.insert(0,str(Path(__file__).resolve().parent))
import source_layout_lifecycle_fixture as fixture_api
from source_layout_lifecycle_fixture import manager,layout,retire,install,sha


class OrdinaryGroupedRetirement(unittest.TestCase):
    def setUp(self):
        self.fixture=fixture_api.SourcePolicy('test_complete_fixture_passes_without_registry_cache')
        self.addCleanup(self.fixture.doCleanups)
        self.fixture.setUp()

    def grouped(self,external=False):
        if external:
            self.fixture.install_external_tests()
            self.fixture.refresh();self.fixture.verify()
        self.directory=install(self.fixture)
        self.marker=self.fixture.catalog[layout.KEY]
        self.receipt_bytes=(self.fixture.root/layout.receipt_path(self.marker)).read_bytes()
        return self.fixture

    def retire_and_verify(self,external=False):
        f=self.grouped(external)
        owner=deepcopy(f.catalog.get('external_tests',{}).get(self.directory))
        plan=retire(f,self.directory)
        if owner is not None:
            plan['packages'][0].update(external_action='remove',external_tests=owner)
            f.remove_external_tests()
        f.refresh(retire_plan=plan);f.verify();f.refresh();f.verify()
        self.assertEqual(f.catalog[layout.KEY],self.marker)
        self.assertEqual((f.root/layout.receipt_path(self.marker)).read_bytes(),self.receipt_bytes)
        self.assertNotIn(self.directory,f.catalog['packages'])
        self.assertIn(self.directory,f.catalog['retired'])
        self.assertFalse(f.source.exists())
        return f

    def test_pre_layout_explicit_retirement_remains_supported(self):
        f=self.fixture;plan=retire(f,'src/foundation/mini')
        f.refresh(retire_plan=plan);f.verify();f.refresh();f.verify()

    def test_grouped_explicit_retirement_refresh_verify_and_repeat(self):
        self.retire_and_verify()

    def test_grouped_external_retirement_keeps_exact_last_labels(self):
        f=self.retire_and_verify(external=True)
        owner=f.catalog['retired'][self.directory]['retirement']['external_tests']
        self.assertEqual(owner['files'][0]['source'],'src/foundation/mini/lib.rs')
        self.assertEqual(owner['directory'],self.directory)
        self.assertEqual(owner['file_count'],1)
        self.assertEqual(f.catalog['external_tests'],{})

    def test_missing_grouped_owner_without_explicit_plan_rejected(self):
        f=self.grouped();retire(f,self.directory)
        with self.assertRaises(ValueError):f.refresh()

    def test_grouped_retirement_stale_package_snapshot_rejected(self):
        f=self.grouped();plan=retire(f,self.directory)
        plan['packages'][0]['package']['name']='forged'
        with self.assertRaises(ValueError):f.refresh(retire_plan=plan)

    def test_retired_grouped_root_cannot_return(self):
        f=self.retire_and_verify();f.source.mkdir()
        with self.assertRaises(ValueError):f.verify()

    def test_retired_old_logical_root_cannot_return(self):
        f=self.retire_and_verify();(f.root/'src/foundation/mini').mkdir()
        with self.assertRaises(ValueError):f.verify()

    def test_rehashed_retirement_identity_mutation_rejected(self):
        f=self.retire_and_verify();f.catalog['retired'][self.directory]['upstream']['version']='9.9.9'
        f.catalog['retired_records_sha256']=manager.origin_hashes(f.catalog['retired']);f.save_catalog()
        with self.assertRaises(ValueError):f.verify()

    def test_unmapped_retired_record_cannot_hide_in_baseline_hash_filter(self):
        f=self.retire_and_verify();unknown='src/foundation/core-types/not-reviewed'
        forged=deepcopy(f.catalog['retired'][self.directory]);forged['directory']=unknown
        f.catalog['retired'][unknown]=forged;f.catalog['retired_records_sha256']=manager.origin_hashes(f.catalog['retired']);f.save_catalog()
        with self.assertRaises(ValueError):f.verify()

    def test_external_retirement_missing_explicit_disposition_rejected(self):
        f=self.grouped(external=True);plan=retire(f,self.directory);f.remove_external_tests()
        with self.assertRaises(ValueError):f.refresh(retire_plan=plan)

    def test_rehashed_retired_external_source_label_change_rejected(self):
        f=self.retire_and_verify(external=True)
        f.catalog['retired'][self.directory]['retirement']['external_tests']['files'][0]['source']='src/foundation/mini/forged.rs'
        f.catalog['retired_records_sha256']=manager.origin_hashes(f.catalog['retired']);f.save_catalog()
        with self.assertRaises(ValueError):f.verify()

    def test_active_and_retired_duplicate_identity_rejected(self):
        f=self.retire_and_verify();package=deepcopy(f.catalog['retired'][self.directory]);package.pop('retirement')
        f.catalog['packages'][self.directory]=package;f.save_catalog()
        with self.assertRaises(ValueError):f.verify()

    def move_and_verify(self):
        f=self.grouped();plan=f.internal_move_plan()
        f.refresh(move_plan_data=plan);f.verify();f.refresh();f.verify()
        self.assertEqual(f.catalog[layout.KEY],self.marker)
        self.assertEqual((f.root/layout.receipt_path(self.marker)).read_bytes(),self.receipt_bytes)
        self.assertFalse((f.source/'lib.rs').exists())
        self.assertTrue((f.source/'value.rs').is_file())
        return f

    def test_grouped_internal_move_refresh_verify_and_repeat(self):
        self.move_and_verify()

    def test_successive_grouped_internal_moves_preserve_old_receipts(self):
        f=self.move_and_verify();before=deepcopy(f.catalog['source_moves'])
        plan=f.internal_move_plan('value.rs','codec.rs')
        f.refresh(move_plan_data=plan);f.verify();f.refresh();f.verify()
        self.assertEqual(len(f.catalog['source_moves']),len(before)+1)
        for key,row in before.items():self.assertEqual(f.catalog['source_moves'][key],row)

    def test_grouped_internal_move_requires_explicit_plan(self):
        f=self.grouped();f.internal_move_plan()
        with self.assertRaises(ValueError):f.refresh()

    def test_grouped_internal_move_stale_identity_rejected(self):
        f=self.grouped();plan=f.internal_move_plan()
        plan['packages'][0]['package']['name']='forged'
        with self.assertRaises(ValueError):f.refresh(move_plan_data=plan)

    def test_grouped_internal_move_missing_successor_rejected(self):
        f=self.move_and_verify();(f.source/'value.rs').unlink()
        with self.assertRaises(ValueError):f.verify()

    def test_rehashed_internal_move_identity_change_rejected(self):
        f=self.move_and_verify();key,row=next(iter(f.catalog['source_moves'].items()))
        changed=deepcopy(row);changed['package']['upstream']['version']='9.9.9'
        del f.catalog['source_moves'][key]
        f.catalog['source_moves'][manager.source_moves.receipt_id(changed)]=changed;f.save_catalog()
        with self.assertRaises(ValueError):f.verify()

    def test_unmapped_internal_move_cannot_hide_in_baseline_hash_filter(self):
        f=self.move_and_verify();row=deepcopy(next(iter(f.catalog['source_moves'].values())))
        row['directory']='src/foundation/core-types/not-reviewed';row['package']['directory']=row['directory']
        f.catalog['source_moves'][manager.source_moves.receipt_id(row)]=row;f.save_catalog()
        with self.assertRaises(ValueError):f.verify()

    def test_new_internal_move_does_not_authorize_old_history_mutation(self):
        f=self.move_and_verify();f.catalog['upstream_identities'][0]['version']='9.9.9';f.save_catalog()
        with self.assertRaises(ValueError):f.verify()

    def test_dependency_lock_may_change_after_installation(self):
        f=self.grouped();lock=f.root/'Cargo.lock';lock.write_bytes(lock.read_bytes()+b'\n')
        f.refresh();f.verify();f.refresh();f.verify()
        self.assertEqual(f.catalog[layout.KEY],self.marker)

    def test_installation_still_requires_exact_predecessor_lock(self):
        f=self.grouped();lock=f.root/'Cargo.lock';lock.write_bytes(lock.read_bytes()+b'\n')
        current=layout.load(f.root,f.catalog)
        with self.assertRaises(ValueError):
            current.verify_installation(f.layout_previous,f.layout_initial_current,f.layout_predecessor_sha)

    def baseline_move_then_grouped_move(self):
        f=self.fixture;plan=f.internal_move_plan()
        f.refresh(move_plan_data=plan);f.verify()
        baseline=deepcopy(f.catalog['source_moves'])
        self.grouped();plan=f.internal_move_plan('value.rs','codec.rs')
        f.refresh(move_plan_data=plan);f.verify();f.refresh();f.verify()
        return f,baseline

    def test_new_grouped_move_preserves_existing_pre_layout_move_receipt(self):
        f,baseline=self.baseline_move_then_grouped_move()
        self.assertEqual(len(f.catalog['source_moves']),len(baseline)+1)
        for key,row in baseline.items():self.assertEqual(f.catalog['source_moves'][key],row)

    def test_rehashed_pre_layout_move_receipt_change_is_not_filtered_as_new(self):
        f,baseline=self.baseline_move_then_grouped_move();key=next(iter(baseline))
        changed=deepcopy(f.catalog['source_moves'].pop(key));changed['reason']+=' rewritten'
        f.catalog['source_moves'][manager.source_moves.receipt_id(changed)]=changed;f.save_catalog()
        with self.assertRaises(ValueError):f.verify()

    def test_grouped_internal_move_can_be_followed_by_explicit_retirement(self):
        f=self.move_and_verify();moves=deepcopy(f.catalog['source_moves'])
        plan=retire(f,self.directory);f.refresh(retire_plan=plan);f.verify();f.refresh();f.verify()
        self.assertEqual(f.catalog['source_moves'],moves)
        self.assertEqual(f.catalog[layout.KEY],self.marker)

    def grouped_with_edited_source(self):
        f=self.fixture
        (f.root/'docs/notes.txt').write_text('Retained documentation.\n')
        (f.source/'lib.rs').write_bytes(b'pub fn value() -> u8 { 1 }\nconst DOC: &str = include_str!("../../../docs/notes.txt");\n')
        f.refresh();f.verify();self.grouped()
        view=layout.load(f.root,f.catalog)
        self.assertIn('src/foundation/mini/lib.rs',view.edits)
        return f

    def test_path_edited_source_move_requires_explicit_descendant_inverse(self):
        f=self.grouped_with_edited_source();plan=f.internal_move_plan()
        with self.assertRaisesRegex(ValueError,'path-edited|descendant'):
            f.refresh(move_plan_data=plan)
        # The old proof view must not return stored before-bytes or silently
        # search the moved successor after the missing live file is detected.
        view=layout.load(f.root,f.catalog)
        with self.assertRaises((ValueError,OSError)):
            view.read('src/foundation/mini/lib.rs')

    def test_rehashed_path_edited_source_move_cannot_bypass_preflight(self):
        f=self.grouped_with_edited_source();record=f.internal_move_plan()['packages'][0]
        f.catalog['source_moves'][manager.source_moves.receipt_id(record)]=record
        f.catalog['packages'][self.directory]['files_sha256']=record['files_sha256'];f.save_catalog()
        with self.assertRaisesRegex(ValueError,'path-edited|descendant'):
            layout.load(f.root,f.catalog)

    def test_all_live_proof_pinned_owners_reject_unadapted_lifecycle(self):
        root=fixture_api.ROOT;catalog=json.loads((root/manager.CATALOG).read_bytes())
        view=layout.load(root,catalog)
        with layout.activate(view):
            for name in ('ring','libc','memchr','mio','rustc-demangle','tokio','system-entropy','synchronization'):
                directory=view.current_path('src/foundation/'+name)
                self.assertIn(directory,catalog['packages'])
                plan=dict(schema=1,packages=[dict(directory=directory,package=deepcopy(catalog['packages'][directory]))])
                for kind in ('retirement','moves'):
                    with self.subTest(name=name,kind=kind),self.assertRaisesRegex(ValueError,'descendant'):
                        view.require_supported_plan(catalog,**{kind:plan})

    def test_authenticated_move_destination_is_visible_to_logical_read_and_inventory(self):
        f=self.move_and_verify();view=layout.load(f.root,f.catalog);old='src/foundation/mini'
        self.assertEqual(view.read(old+'/value.rs'),(f.source/'value.rs').read_bytes())
        expected={old+'/'+path:value for path,value in f.catalog['packages'][self.directory]['files_sha256'].items()}
        self.assertEqual(view.inventory(old),expected)
        with self.assertRaises((ValueError,OSError)):view.read(old+'/lib.rs')

    def test_move_descendant_allowlist_does_not_admit_unknown_adjacent_file(self):
        f=self.move_and_verify();view=layout.load(f.root,f.catalog)
        (f.source/'not-reviewed.rs').write_bytes(b'pub fn not_reviewed() {}\n')
        with self.assertRaises(ValueError):view.read('src/foundation/mini/not-reviewed.rs')
        with self.assertRaises(ValueError):view.inventory('src/foundation/mini')

    def test_authenticated_move_destination_rechecks_live_content_hash(self):
        f=self.move_and_verify();view=layout.load(f.root,f.catalog)
        path=f.source/'value.rs';path.write_bytes(path.read_bytes()+b'// changed after authentication\n')
        with self.assertRaises(ValueError):view.read('src/foundation/mini/value.rs')

    def test_successive_move_inventory_exposes_only_current_authenticated_descendants(self):
        f=self.move_and_verify();plan=f.internal_move_plan('value.rs','codec.rs')
        f.refresh(move_plan_data=plan);f.verify();view=layout.load(f.root,f.catalog)
        self.assertEqual(view.read('src/foundation/mini/codec.rs'),(f.source/'codec.rs').read_bytes())
        self.assertNotIn('src/foundation/mini/value.rs',view.inventory('src/foundation/mini'))
        (f.source/'value.rs').write_bytes((f.source/'codec.rs').read_bytes())
        with self.assertRaises(ValueError):view.read('src/foundation/mini/value.rs')

    def test_source_only_move_preserves_exact_existing_external_attribution(self):
        f=self.grouped(external=True);before=deepcopy(f.catalog['external_tests'][self.directory])
        plan=f.internal_move_plan();f.refresh(move_plan_data=plan);f.verify();f.refresh();f.verify()
        self.assertEqual(f.catalog['external_tests'][self.directory],before)

    def test_external_file_move_reports_explicit_unsupported_descendant_scope(self):
        f=self.grouped(external=True);plan=f.internal_move_plan(move_external=True)
        with self.assertRaisesRegex(ValueError,'external file moves.*descendant'):
            f.refresh(move_plan_data=plan)


class ProjectAuthoredGroupedTests(unittest.TestCase):
    setUp = OrdinaryGroupedRetirement.setUp
    grouped = OrdinaryGroupedRetirement.grouped

    def add_local_test(self, f):
        path = f.external_root / 'regression.rs'
        path.write_bytes(b'#[test]\nfn regression() { assert_eq!(2 + 2, 4); }\n')
        relative = path.relative_to(f.root).as_posix()
        row = dict(source_kind='project-authored', source=relative, path=relative,
                   source_sha256_before=sha(path.read_bytes()), sha256=sha(path.read_bytes()))
        mapping = json.loads(f.external_mapping.read_bytes())
        mapping['owners'][0]['files'].append(row)
        f.external_mapping.write_text(json.dumps(mapping))
        return path, mapping

    def test_local_addition_refresh_verify_and_content_edit_keep_layout_evidence(self):
        f = self.grouped(external=True)
        before = deepcopy(f.catalog)
        path, mapping = self.add_local_test(f)
        initial = deepcopy(mapping['owners'][0]['files'][-1])
        f.refresh(); f.verify(); f.refresh(); f.verify()
        owner = f.catalog['external_tests'][self.directory]
        self.assertEqual(owner['file_count'], 2)
        self.assertEqual(owner['files'][0], before['external_tests'][self.directory]['files'][0])
        self.assertEqual(owner['files'][-1], initial)
        path.write_bytes(path.read_bytes() + b'// reviewed later regression coverage\n')
        mapping['owners'][0]['files'][-1]['sha256'] = sha(path.read_bytes())
        f.external_mapping.write_text(json.dumps(mapping))
        f.refresh(); f.verify()
        current = f.catalog['external_tests'][self.directory]['files'][-1]
        self.assertNotEqual(current['sha256'], initial['sha256'])
        self.assertEqual({k: v for k, v in current.items() if k != 'sha256'},
                         {k: v for k, v in initial.items() if k != 'sha256'})
        self.assertEqual((f.root / layout.receipt_path(self.marker)).read_bytes(), self.receipt_bytes)
        for field in layout.HISTORY_FIELDS:
            self.assertEqual(f.catalog.get(field), before.get(field), field)
        view = layout.load(f.root, f.catalog)
        self.assertEqual(view.rows['src/foundation/mini']['destination_external_tests'],
                         before['external_tests'][self.directory])
        self.assertNotIn(initial['path'], view.edits)

    def test_local_addition_does_not_allow_unmarked_historical_additions(self):
        f = self.grouped(external=True)
        _, mapping = self.add_local_test(f)
        row = mapping['owners'][0]['files'][-1]
        del row['source_kind']
        row['source'] = 'src/foundation/mini/lib.rs'
        f.external_mapping.write_text(json.dumps(mapping))
        with self.assertRaisesRegex(ValueError, 'external historical attribution drift'):
            f.refresh()

    def test_local_addition_cannot_hide_old_label_rewrites_or_drops(self):
        f = self.grouped(external=True)
        self.add_local_test(f); f.refresh(); f.verify()
        original = deepcopy(f.catalog)
        for field, value in [('source', 'src/foundation/mini/forged.rs'),
                             ('source_sha256_before', 'a' * 64),
                             ('path', 'test/foundation/mini/forged.rs'),
                             ('source_kind', 'project-authored'), ('drop', None)]:
            with self.subTest(field=field):
                catalog = deepcopy(original)
                owner = catalog['external_tests'][self.directory]
                if field == 'drop': owner['files'].pop(0); owner['file_count'] -= 1
                else: owner['files'][0][field] = value
                with self.assertRaisesRegex(ValueError, 'external historical attribution drift'):
                    layout.load(f.root, catalog)

    def test_added_local_rows_cannot_change_owner_roots_or_forge_count(self):
        f = self.grouped(external=True)
        self.add_local_test(f); f.refresh(); f.verify()
        original = deepcopy(f.catalog)
        for field, value in [('directory', 'src/foundation/core-types/other'),
                             ('origin_directory', 'old/other'),
                             ('roots', ['test/foundation']), ('file_count', 1)]:
            with self.subTest(field=field):
                catalog = deepcopy(original); catalog['external_tests'][self.directory][field] = value
                with self.assertRaisesRegex(ValueError, 'external historical attribution drift'):
                    layout.load(f.root, catalog)

    def test_added_local_rows_require_own_source_scope_in_layout_check(self):
        f = self.grouped(external=True)
        self.add_local_test(f); f.refresh(); f.verify()
        original = deepcopy(f.catalog)
        for field, value in [('source', 'src/foundation/mini/lib.rs'),
                             ('source', 'test/foundation/other/regression.rs'),
                             ('origin_directory', 'old/other')]:
            with self.subTest(field=field, value=value):
                catalog = deepcopy(original)
                catalog['external_tests'][self.directory]['files'][-1][field] = value
                with self.assertRaisesRegex(ValueError, 'external historical attribution drift'):
                    layout.load(f.root, catalog)

    def test_added_local_initial_hash_and_disappearance_still_fail_refresh(self):
        f = self.grouped(external=True)
        path, mapping = self.add_local_test(f); f.refresh(); f.verify()
        original = deepcopy(mapping)
        mapping['owners'][0]['files'][-1]['source_sha256_before'] = 'a' * 64
        f.external_mapping.write_text(json.dumps(mapping))
        with self.assertRaisesRegex(ValueError, 'accounting shrank'):
            f.refresh()
        original['owners'][0]['files'].pop(); path.unlink()
        f.external_mapping.write_text(json.dumps(original))
        with self.assertRaisesRegex(ValueError, 'accounting shrank'):
            f.refresh()

    def test_local_rows_already_in_installation_remain_historical_labels(self):
        f = self.fixture
        f.install_external_tests()
        self.add_local_test(f); f.refresh(); f.verify()
        self.grouped()
        original = deepcopy(f.catalog)
        for field, value in [('source_kind', 'other'), ('source_sha256_before', 'a' * 64),
                             ('path', 'test/foundation/mini/other.rs'), ('drop', None)]:
            with self.subTest(field=field):
                catalog = deepcopy(original)
                owner = catalog['external_tests'][self.directory]
                if field == 'drop': owner['files'].pop(); owner['file_count'] -= 1
                else: owner['files'][-1][field] = value
                with self.assertRaisesRegex(ValueError, 'external historical attribution drift'):
                    layout.load(f.root, catalog)


if __name__=='__main__':unittest.main(verbosity=2)
