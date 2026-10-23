"""Adversarial append-only correction of nested external compiler include paths."""
from copy import deepcopy
import gzip
import importlib
import json
import os
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

HERE=Path(__file__).resolve().parent
ROOT=Path(os.environ.get('SOURCE_LAYOUT_REPOSITORY',str(HERE.parents[2] if len(HERE.parents)>=3 else Path.cwd())))
sys.path.insert(0,str(ROOT/'tools/rust-sources'))
sys.path.insert(0,str(ROOT/'test/tools/rust-sources'))
from test_source_layout import LayoutFixture,layout,make_receipt
from source_layout_fixture import digest,encoded,material
import source_layout_corrections as corrections


class CorrectionFixture(LayoutFixture):
    def setUp(self):
        super().setUp()
        self.old='src/foundation/alpha';self.owner=self.scenario.routes[self.old]
        self.path='test/foundation/old-origin/modules/deep/nested.rs'
        self.target='test/foundation/old-origin/functions/leaf.rs'
        self.before_literal='/../../../'+self.target
        self.after_literal='/../../../../'+self.target
        self.before=('mod nested { include!(concat!(env!("CARGO_MANIFEST_DIR"), "'+self.before_literal+'")); }\n').encode()
        self.after=self.before.replace(self.before_literal.encode(),self.after_literal.encode())
        leaves={self.path:self.before,self.target:b'#[test]\nfn leaf() { assert_eq!(2 + 2, 4); }\n'}
        for path,data in leaves.items():
            self.scenario.before_bytes[path]=data;self.scenario.after_bytes[path]=data
        material(self.root,leaves)
        for cat,key in [(self.scenario.previous,self.old),(self.scenario.current,self.owner)]:
            owner=cat['external_tests'][key]
            owner['roots'].append('test/foundation/old-origin')
            owner['file_count']+=len(leaves);owner['bytes']+=sum(map(len,leaves.values()))
            for path,data in leaves.items():
                owner['files'].append(dict(path=path,source='src/foundation/alpha/src/tests.rs',
                    sha256=digest(data),source_sha256_before=digest(b'exact historical upstream module')))
        self.receipt=make_receipt(self.scenario);self.pin()
        self.original_receipt_path=self.root/layout.receipt_path(self.receipt_id)
        self.original_receipt_bytes=self.original_receipt_path.read_bytes()
        self.before_catalog=deepcopy(self.scenario.current)
        self.base_layout=self.install()
        start=self.before.index(self.before_literal.encode())
        self.correction_receipt=dict(schema=1,layout_receipt_id=self.receipt_id,
            predecessor_catalog_sha256=digest(encoded(self.before_catalog)),files=[dict(
                path=self.path,owner=self.owner,before_sha256=digest(self.before),after_sha256=digest(self.after),
                before_size=len(self.before),after_size=len(self.after),references=[dict(
                    start=start,end=start+len(self.before_literal),before=self.before_literal,
                    after=self.after_literal,target=self.target)])])
        self.correction_pin=patch.object(corrections,'PINNED_RECEIPTS',())
        self.correction_pin.start();self.addCleanup(self.correction_pin.stop)
        self.write_after()
        self.pin_correction()

    def write_after(self):
        (self.root/self.path).write_bytes(self.after)
        owner=self.scenario.current['external_tests'][self.owner]
        owner['bytes']=self.before_catalog['external_tests'][self.owner]['bytes']+len(self.after)-len(self.before)
        next(x for x in owner['files'] if x['path']==self.path)['sha256']=digest(self.after)

    def pin_correction(self):
        raw=gzip.compress(encoded(self.correction_receipt),mtime=0);self.correction_id=digest(raw)
        path=self.root/corrections.receipt_path(self.correction_id);path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(raw)
        corrections.PINNED_RECEIPTS=(self.correction_id,)
        self.scenario.current[corrections.KEY]=self.correction_id
        self.scenario.write_current()

    def correction(self):
        return corrections.load(self.root,self.base_layout,self.scenario.current)

    def row(self):return self.correction_receipt['files'][0]

    def reset_expression(self, before):
        self.scenario.current=deepcopy(self.before_catalog)
        old_size=len(self.before);self.before=before
        self.after=before.replace(self.before_literal.encode(),self.after_literal.encode())
        for cat,key in [(self.scenario.previous,self.old),(self.scenario.current,self.owner)]:
            owner=cat['external_tests'][key];owner['bytes']+=len(before)-old_size
            next(x for x in owner['files'] if x['path']==self.path)['sha256']=digest(before)
        self.scenario.before_bytes[self.path]=before;self.scenario.after_bytes[self.path]=before
        (self.root/self.path).write_bytes(before)
        self.receipt=make_receipt(self.scenario);self.pin()
        self.original_receipt_path=self.root/layout.receipt_path(self.receipt_id)
        self.original_receipt_bytes=self.original_receipt_path.read_bytes()
        self.before_catalog=deepcopy(self.scenario.current)
        corrections.PINNED_RECEIPTS=()
        self.base_layout=self.install()
        start=before.index(self.before_literal.encode())
        self.correction_receipt.update(layout_receipt_id=self.receipt_id,
            predecessor_catalog_sha256=digest(encoded(self.before_catalog)))
        self.row().update(before_sha256=digest(before),after_sha256=digest(self.after),
            before_size=len(before),after_size=len(self.after),references=[dict(start=start,
            end=start+len(self.before_literal),before=self.before_literal,after=self.after_literal,target=self.target)])
        self.write_after();self.pin_correction()


class NestedExternalCorrections(CorrectionFixture):
    def test_nested_stationary_module_rebases_from_authenticated_owner(self):
        view=self.correction()
        self.assertEqual(view.read(self.path,self.after),self.before)
        self.assertTrue((self.root/self.owner/self.after_literal.lstrip('/')).is_file())
        self.assertEqual(self.original_receipt_path.read_bytes(),self.original_receipt_bytes)

    def test_current_hash_and_byte_delta_project_exactly_to_before(self):
        view=self.correction()
        self.assertEqual(view.project_hash(self.path,digest(self.after)),digest(self.before))
        self.assertEqual(view.delta(self.path,digest(self.after)),len(self.after)-len(self.before))
        self.assertEqual(view.project_hash(self.path,'f'*64),'f'*64)
        self.assertEqual(view.delta(self.path,'f'*64),0)

    def test_integrated_layout_reads_live_after_then_returns_original_proof_bytes(self):
        view=layout.load(self.root,self.scenario.current)
        self.assertEqual(view.read(self.path),self.before)
        projected=view.project_catalog(self.scenario.current)
        self.assertEqual(projected['external_tests'],self.scenario.previous['external_tests'])
        self.assertEqual(self.original_receipt_path.read_bytes(),self.original_receipt_bytes)

    def test_corrected_live_bytes_tamper_fails_before_inverse(self):
        view=self.correction()
        with self.assertRaises(ValueError):view.read(self.path,self.after+b'\n')

    def test_missing_live_file_has_no_receipt_archive_fallback(self):
        (self.root/self.path).unlink()
        with self.assertRaises(ValueError):layout.load(self.root,self.scenario.current).read(self.path)

    def test_missing_target_fails_closed(self):
        (self.root/self.target).unlink()
        with self.assertRaises(ValueError):self.correction()

    def test_owner_is_catalog_identity_not_test_directory_name(self):
        self.row()['owner']='src/foundation/platform/plain';self.pin_correction()
        with self.assertRaises(ValueError):self.correction()

    def test_duplicate_external_claim_rejected(self):
        duplicate=deepcopy(self.scenario.current['external_tests'][self.owner])
        duplicate['directory']='src/foundation/platform/plain'
        self.scenario.current['external_tests'][duplicate['directory']]=duplicate
        self.scenario.write_current()
        with self.assertRaises(ValueError):self.correction()

    def test_target_cannot_be_substituted_for_other_existing_file(self):
        self.row()['references'][0]['target']='test/foundation/alpha/tests/smoke.rs';self.pin_correction()
        with self.assertRaises(ValueError):self.correction()

    def test_before_must_match_immutable_published_layout_inventory(self):
        self.row()['before_sha256']='a'*64;self.pin_correction()
        with self.assertRaises(ValueError):self.correction()

    def test_self_consistent_hidden_semantic_change_cannot_be_corrected_path(self):
        self.after=self.after.replace(b'mod nested',b'mod malicious')
        self.row()['after_sha256']=digest(self.after);self.row()['after_size']=len(self.after)
        self.write_after();self.pin_correction()
        with self.assertRaises(ValueError):self.correction()

    def test_offset_cannot_rewrite_comment_or_unrelated_string(self):
        self.row()['references'][0]['start']=0
        self.row()['references'][0]['end']=len(self.before_literal)
        self.pin_correction()
        with self.assertRaises(ValueError):self.correction()

    def test_duplicate_file_receipt_rejected(self):
        self.correction_receipt['files'].append(deepcopy(self.row()));self.pin_correction()
        with self.assertRaises(ValueError):self.correction()

    def test_duplicate_reference_rejected(self):
        self.row()['references'].append(deepcopy(self.row()['references'][0]));self.pin_correction()
        with self.assertRaises(ValueError):self.correction()

    def test_unpinned_receipt_rejected(self):
        corrections.PINNED_RECEIPTS=()
        with self.assertRaises(ValueError):self.correction()

    def test_wrong_parent_layout_pin_rejected(self):
        self.correction_receipt['layout_receipt_id']='f'*64;self.pin_correction()
        with self.assertRaises(ValueError):self.correction()

    def test_compressed_receipt_tamper_rejected(self):
        p=self.root/corrections.receipt_path(self.correction_id);p.write_bytes(p.read_bytes()+b'junk')
        with self.assertRaises(ValueError):self.correction()

    def test_live_catalog_hash_must_equal_reviewed_after(self):
        next(x for x in self.scenario.current['external_tests'][self.owner]['files'] if x['path']==self.path)['sha256']='e'*64
        self.scenario.write_current()
        with self.assertRaises(ValueError):self.correction()

    def test_original_source_label_and_source_hash_remain_exact(self):
        row=next(x for x in self.scenario.current['external_tests'][self.owner]['files'] if x['path']==self.path)
        row['source_sha256_before']='f'*64;self.scenario.write_current()
        with self.assertRaises(ValueError):layout.load(self.root,self.scenario.current)

    def direct_pinned_reader(self):
        historical=importlib.import_module('historical_test_sources')
        routing=importlib.import_module('proof_routing')
        proof={'schema_version':1,'files':[dict(source=self.path,path=self.path,
            sha256=digest(self.before),before_sha256=digest(self.before))]}
        raw=encoded(proof);material(self.root,{historical.CORRECTIONS:raw})
        return historical,routing,raw

    def test_existing_pinned_proof_reader_sees_exact_original_external_bytes(self):
        historical,routing,raw=self.direct_pinned_reader()
        view=layout.load(self.root,self.scenario.current)
        with layout.activate(view),patch.object(historical,'CORRECTIONS_SHA256',digest(raw)):
            original,rows=historical.corrected_view(self.root,routing.View(self.root))
            self.assertEqual(original[self.path],self.before)
            self.assertEqual(rows[0]['sha256'],digest(self.before))
            self.assertEqual((self.root/historical.CORRECTIONS).read_bytes(),raw)

    def test_existing_pinned_proof_reader_cannot_hide_late_live_tamper(self):
        historical,routing,raw=self.direct_pinned_reader()
        view=layout.load(self.root,self.scenario.current)
        (self.root/self.path).write_bytes(self.after+b'// hidden change\n')
        with layout.activate(view),patch.object(historical,'CORRECTIONS_SHA256',digest(raw)):
            with self.assertRaises(ValueError):historical.corrected_view(self.root,routing.View(self.root))

    def test_exact_expression_inside_comment_cannot_be_certified(self):
        self.reset_expression(b'// '+self.before)
        with self.assertRaises(ValueError):self.correction()

    def test_exact_expression_inside_raw_string_cannot_be_certified(self):
        self.reset_expression(b'let example=r#"'+self.before.rstrip()+b'"#;\n')
        with self.assertRaises(ValueError):self.correction()

    def test_explicit_install_plan_preserves_parent_receipt_and_labels(self):
        view,current=corrections.prepare(self.root,self.before_catalog,
            {'schema':1,'receipt_id':self.correction_id},digest(encoded(self.before_catalog)))
        self.assertEqual(view.read(self.path),self.before)
        self.assertEqual(current,self.scenario.current)
        self.assertEqual(self.original_receipt_path.read_bytes(),self.original_receipt_bytes)

    def test_stale_explicit_plan_rejected(self):
        with self.assertRaises(ValueError):corrections.prepare(self.root,self.before_catalog,
            {'schema':1,'receipt_id':self.correction_id},'e'*64)

    def test_missing_explicit_plan_rejected(self):
        with self.assertRaises(ValueError):corrections.prepare(self.root,self.before_catalog,{},
            digest(encoded(self.before_catalog)))

    def test_correction_catalog_marker_cannot_be_removed(self):
        self.scenario.current.pop(corrections.KEY);self.scenario.write_current()
        with self.assertRaises(ValueError):layout.load(self.root,self.scenario.current)

    def test_published_parent_receipt_tamper_still_rejected(self):
        self.original_receipt_path.write_bytes(self.original_receipt_bytes+b'x')
        with self.assertRaises(ValueError):layout.load(self.root,self.scenario.current)

    def test_old_consolidation_history_tamper_still_rejected(self):
        self.scenario.current['consolidations']['invented']={'external_file_routes':{}}
        self.scenario.write_current()
        with self.assertRaises(ValueError):layout.load(self.root,self.scenario.current)

    def test_live_correction_file_symlink_rejected(self):
        path=self.root/self.path;path.unlink();path.symlink_to(self.root/self.target)
        with self.assertRaises(ValueError):self.correction()

    def test_target_symlink_rejected(self):
        path=self.root/self.target;path.unlink();path.symlink_to(self.root/'test/foundation/alpha/tests/smoke.rs')
        with self.assertRaises(ValueError):self.correction()

    def test_unknown_adjacent_file_is_not_implicitly_inverse_transformed(self):
        view=self.correction()
        other='test/foundation/old-origin/modules/deep/adjacent.rs'
        self.assertEqual(view.read(other,self.after),self.after)
        self.assertEqual(view.project_hash(other,digest(self.after)),digest(self.after))

class CorrectedOwnerLifecycle(CorrectionFixture):
    def plan(self,directory,moves=False):
        item={'directory':directory}
        if moves:item.update(file_moves={'src/value.rs':'src/renamed.rs'},external_file_moves={})
        return {'schema':1,'packages':[item]}

    def test_corrected_owner_retirement_preflight_rejects_explicitly(self):
        view=layout.load(self.root,self.scenario.current)
        with self.assertRaisesRegex(ValueError,'corrected external owner lifecycle'):
            view.require_supported_plan(self.scenario.current,retirement=self.plan(self.owner))

    def test_corrected_owner_internal_move_preflight_rejects_explicitly(self):
        view=layout.load(self.root,self.scenario.current)
        with self.assertRaisesRegex(ValueError,'corrected external owner lifecycle'):
            view.require_supported_plan(self.scenario.current,moves=self.plan(self.owner,True))

    def move_record(self,directory,source,destination):
        package=deepcopy(self.scenario.current['packages'][directory])
        after=deepcopy(package['files_sha256']);after[destination]=after.pop(source)
        external=deepcopy(self.scenario.current['external_tests'].get(directory))
        record=dict(directory=directory,package=package,merged_packages=deepcopy([
            x for x in self.scenario.current['merges'] if x['directory']==directory]),
            files_sha256=after,file_moves={source:destination},external_file_moves={},
            external_tests=external,destination_external_tests=deepcopy(external),reason='Reviewed source-only internal move')
        return record

    def test_forged_appended_corrected_owner_move_cannot_bypass_preflight(self):
        import source_moves
        record=self.move_record(self.owner,'src/value.rs','src/renamed.rs')
        self.scenario.current['source_moves'][source_moves.receipt_id(record)]=record
        self.scenario.write_current()
        with self.assertRaisesRegex(ValueError,'corrected external owner lifecycle'):
            layout.load(self.root,self.scenario.current)

    def retire_record(self,directory):
        import shutil
        package=self.scenario.current['packages'].pop(directory)
        files=list((self.root/directory).rglob('*'))
        retirement=dict(reason='Reviewed standalone retirement',category='platform',
            bytes=sum(p.stat().st_size for p in files if p.is_file()),
            file_count=len(package['files_sha256']),targets=['x86_64-unknown-linux-gnu'])
        external=self.scenario.current['external_tests'].pop(directory,None)
        if external is not None:retirement['external_tests']=external
        package['retirement']=retirement
        self.scenario.current['retired'][directory]=package
        self.scenario.current['retired_records_sha256'][directory]=digest(encoded(package))
        self.scenario.current['merges']=[x for x in self.scenario.current['merges'] if x['directory']!=directory]
        shutil.rmtree(self.root/directory)
        self.scenario.write_current()

    def test_forged_rehashed_corrected_owner_retirement_cannot_bypass_preflight(self):
        self.retire_record(self.owner)
        with self.assertRaisesRegex(ValueError,'corrected external owner lifecycle'):
            layout.load(self.root,self.scenario.current)

    def test_corrected_owner_attribution_corruption_is_not_a_lifecycle_escape(self):
        self.scenario.current['external_tests'][self.owner]['directory']='src/foundation/forged'
        self.scenario.write_current()
        with self.assertRaisesRegex(ValueError,'external historical attribution drift'):
            layout.load(self.root,self.scenario.current)

    def test_unrelated_standalone_owner_retirement_and_move_preflight_remain_allowed(self):
        plain=self.scenario.routes['src/foundation/plain']
        view=layout.load(self.root,self.scenario.current)
        view.require_supported_plan(self.scenario.current,retirement=self.plan(plain))
        view.require_supported_plan(self.scenario.current,moves={
            'schema':1,'packages':[dict(directory=plain,file_moves={'src/lib.rs':'src/renamed.rs'},external_file_moves={})]})
        self.assertEqual(view.read(self.path),self.before)

    def test_unrelated_standalone_owner_retirement_remains_loadable(self):
        plain=self.scenario.routes['src/foundation/plain']
        self.retire_record(plain)
        view=layout.load(self.root,self.scenario.current)
        self.assertEqual(view.read(self.path),self.before)
        self.assertEqual(self.scenario.current[corrections.KEY],self.correction_id)
        self.assertEqual(self.original_receipt_path.read_bytes(),self.original_receipt_bytes)

    def test_unrelated_standalone_owner_internal_move_remains_loadable(self):
        import source_moves
        plain=self.scenario.routes['src/foundation/plain']
        record=self.move_record(plain,'src/lib.rs','src/renamed.rs')
        (self.root/plain/'src/lib.rs').rename(self.root/plain/'src/renamed.rs')
        self.scenario.current['packages'][plain]['files_sha256']=deepcopy(record['files_sha256'])
        self.scenario.current['source_moves'][source_moves.receipt_id(record)]=record
        self.scenario.write_current()
        view=layout.load(self.root,self.scenario.current)
        self.assertEqual(view.read(self.path),self.before)
        self.assertEqual(view.read('src/foundation/plain/src/renamed.rs'),b'pub fn identity(value: u8) -> u8 { value }\n')
        self.assertEqual(self.original_receipt_path.read_bytes(),self.original_receipt_bytes)

if __name__=='__main__':unittest.main(verbosity=2)
