"""Actual bounded proof loading keeps merged foreign files outside old proofs."""
from copy import deepcopy
import gzip
import hashlib
import json
import unittest
from unittest.mock import patch
import test_proof_consolidation as fixture
import proof_routing as routing
import source_consolidation
import external_transfers


def sha(data):return hashlib.sha256(data).hexdigest()


class ConsolidatedProofRouting(unittest.TestCase):
    def setUp(self):
        fixture.ProjectionFixture.setUp(self)
        self.pin()

    def pin(self,rows=None):
        ids=[]
        for row in rows or [self.row]:
            data=gzip.compress(json.dumps(row,sort_keys=True).encode(),mtime=0);h=sha(data);ids.append(h)
            p=self.root/'compliance/proof-routing'/f'{h}.json.gz';p.parent.mkdir(exist_ok=True);p.write_bytes(data)
        stub=patch.object(routing,'RECEIPTS',tuple(ids));stub.start();self.addCleanup(stub.stop)
        self.catalog['proof_routing_receipts']=ids
        return ids

    def view(self):return routing.load(self.root,self.catalog)

    def test_exact_primary_bijection_and_foreign_union_are_both_verified(self):
        view=self.view()
        expected={p:b for p,b in self.before_bytes.items() if p.startswith(self.primary+'/') or p.startswith('test/foundation/mini/')}
        self.assertEqual(view.bytes,expected)
        self.assertNotIn(self.secondary+'/lib.rs',view.bytes)
        self.assertEqual(len(view.forward),3)
        self.assertEqual(view.packages[self.primary]['origin_directory'],'vendor/mini')
        self.assertEqual(routing.validate_refresh(self.root,self.previous,self.catalog).bytes,expected)

    def test_prepare_authorizes_only_fresh_exact_pinned_consolidation(self):
        self.assertTrue(routing.authorize_consolidation(self.root,self.previous,self.record))
        def inventory(path):return {p.relative_to(path).as_posix():sha(p.read_bytes()) for p in path.rglob('*') if p.is_file()}
        args=(self.root,self.previous,dict(schema=1,packages=[self.record]),inventory)
        with self.assertRaisesRegex(ValueError,'historical-view adapter'):
            source_consolidation.prepare(*args,validate_external=external_transfers.validate)
        result=source_consolidation.prepare(*args,validate_external=external_transfers.validate,authorize_pinned=routing.authorize_consolidation)
        self.assertEqual(result[0],self.catalog['packages'])
        wrong=deepcopy(self.record);wrong['reason']+=' wrong plan'
        with self.assertRaisesRegex(ValueError,'origin projection'):
            routing.authorize_consolidation(self.root,self.previous,wrong)

    def test_foreign_changed_missing_extra_symlink_and_empty_rejected(self):
        p=self.root/self.destination/'utils.rs';data=p.read_bytes()
        for changed in (b'',b'changed'):
            p.write_bytes(changed)
            with self.assertRaises(ValueError):self.view()
        p.unlink()
        with self.assertRaises(ValueError):self.view()
        p.write_bytes(data);extra=p.parent/'unclaimed.rs';extra.write_bytes(b'x')
        with self.assertRaises(ValueError):self.view()
        extra.unlink();p.unlink();p.symlink_to(p.parent/'core.rs')
        with self.assertRaises(ValueError):self.view()

    def test_foreign_test_changed_missing_and_extra_rejected(self):
        p=self.root/'test/foundation/combined/utils.rs';data=p.read_bytes();p.write_bytes(b'changed')
        with self.assertRaises(ValueError):self.view()
        p.unlink()
        with self.assertRaises(ValueError):self.view()
        p.write_bytes(data);(p.parent/'unclaimed.rs').write_bytes(b'x')
        with self.assertRaises(ValueError):self.view()

    def test_foreign_file_cannot_enter_primary_inverse_bijection(self):
        self.row['files'][1]['after']=self.destination+'/utils.rs';self.row['files'][1]['after_sha256']=self.record['destination_files_sha256']['utils.rs'];self.pin()
        with self.assertRaisesRegex(ValueError,'bijection/hash'):
            self.view()

    def test_primary_loss_wrong_inverse_and_duplicate_are_rejected(self):
        original=deepcopy(self.row)
        self.row['files'].pop();self.pin()
        with self.assertRaisesRegex(ValueError,'inventory'):self.view()
        self.row=deepcopy(original);self.row['files'][0]['replacements'][0]['old']='d3Jvbmc=';self.pin()
        with self.assertRaises(ValueError):self.view()
        self.row=deepcopy(original);self.row['files'].append(deepcopy(self.row['files'][0]));self.pin()
        with self.assertRaises(ValueError):self.view()

    def test_registered_origin_or_full_owner_drift_rejected(self):
        original=deepcopy(self.catalog)
        self.catalog['merges'][0]['origin_directory']='vendor/mini'
        with self.assertRaises(ValueError):self.view()
        self.catalog=deepcopy(original);self.catalog['packages'][self.destination]['files_sha256'].pop('utils.rs')
        with self.assertRaises(ValueError):self.view()
        self.catalog=deepcopy(original);self.catalog['external_tests'][self.destination]['files'][1]['origin_directory']='vendor/mini'
        with self.assertRaises(ValueError):self.view()

    def test_old_primary_and_secondary_roots_cannot_return(self):
        for relative in (self.primary,self.secondary,'test/foundation/mini','test/foundation/utils'):
            p=self.root/relative;p.mkdir(parents=True)
            with self.assertRaises(ValueError):self.view()
            p.rmdir()

    def test_consolidation_binding_is_terminal_no_later_edit_bypass(self):
        parent=routing.RECEIPTS[0];following=deepcopy(self.row)
        following.update(parent=parent,before=deepcopy(self.row['after']),external_before=deepcopy(self.row['external_after']),binding=dict(kind='edit',record=None))
        for row in following['files']:
            row.update(before=row['after'],before_sha256=row['after_sha256'],replacements=[])
        self.pin([self.row,following])
        with self.assertRaisesRegex(ValueError,'new authenticated adapter'):
            self.view()

    def test_receipt_and_previous_catalog_tamper_rejected(self):
        previous=deepcopy(self.previous);previous['packages'][self.primary]['name']='wrong'
        with self.assertRaises(ValueError):routing.validate_refresh(self.root,previous,self.catalog)
        receipt=self.root/'compliance/proof-routing'/f'{routing.RECEIPTS[0]}.json.gz';receipt.write_bytes(receipt.read_bytes()+b'x')
        with self.assertRaisesRegex(ValueError,'immutable proof routing'):
            self.view()


if __name__=='__main__':unittest.main()
