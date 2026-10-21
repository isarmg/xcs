import base64
from copy import deepcopy
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / 'tools/rust-sources'))
import external_tests
import external_transfers as transfer


def sha(data): return hashlib.sha256(data).hexdigest()
def enc(data): return base64.b64encode(data).decode()


class ExactOriginTransfer(unittest.TestCase):
    def setUp(self):
        t = tempfile.TemporaryDirectory(); self.addCleanup(t.cleanup); self.root = Path(t.name)
        self.destination = 'src/foundation/core'
        self.record = dict(primary_directory='src/foundation/first', destination_directory=self.destination, inputs=[], external_file_routes={})
        self.before = {}; files = []
        for name in ('first', 'second'):
            d = 'src/foundation/' + name; origin = 'vendor/' + name
            data = ('// retained context\nuse '+name+'::Thing;\nassert!(true);\n').encode()
            new = data.replace((name+'::').encode(), b'core::')
            old_path = 'test/foundation/'+name+'/unit.rs'; new_path = 'test/foundation/core/'+name+'/unit.rs'
            package = dict(directory=d, origin_directory=origin, files_sha256={'src/lib.rs':sha(b'pub struct Thing;\n')})
            row = dict(path=old_path, source=d+'/src/lib.rs', source_sha256_before=sha(data), sha256=sha(data))
            owner = dict(directory=d, origin_directory=origin, roots=['test/foundation/'+name], files=[row], file_count=1, bytes=len(data))
            self.before[d] = owner; self.record['inputs'].append(dict(package=package, external_tests=owner))
            files.append(dict(row,path=new_path,sha256=sha(new),origin_directory=origin))
            self.record['external_file_routes'][old_path] = dict(destination=new_path,replacements=[dict(old=enc((name+'::').encode()),new=enc(b'core::'))])
            p=self.root/new_path;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(new)
        self.owner = dict(directory=self.destination,origin_directory='vendor/first',roots=['test/foundation/core'],files=files,file_count=2,bytes=sum((self.root/r['path']).stat().st_size for r in files))
        self.record['destination_external_tests'] = self.owner

    def validate(self, record=None, **kw): transfer.validate(self.root,record or self.record,**kw)
    def test_exact_two_origin_transfer_and_scoped_grants(self):
        self.validate();g=transfer.grants({'receipt':self.record})
        for row in self.owner['files']: self.assertTrue(transfer.permits(g,self.destination,row,'vendor/first'))
        row=dict(self.owner['files'][1],source='src/foundation/second/other.rs')
        self.assertFalse(transfer.permits(g,self.destination,row,'vendor/first'))
        self.assertFalse(transfer.permits(g,'src/foundation/other',self.owner['files'][1],'vendor/first'))

    def test_missing_changed_empty_extra_and_symlink_fail(self):
        row=self.owner['files'][0];p=self.root/row['path'];data=p.read_bytes()
        for value in (b'',b'changed'):
            with self.subTest(value=value):
                p.write_bytes(value)
                with self.assertRaises(ValueError):self.validate()
        p.write_bytes(data);p.unlink()
        with self.assertRaises(ValueError):self.validate()
        p.write_bytes(data);extra=p.parent/'extra';extra.write_bytes(b'x')
        with self.assertRaises(ValueError):self.validate()
        extra.unlink();p.unlink();p.symlink_to(self.root/self.owner['files'][1]['path'])
        with self.assertRaises(ValueError):self.validate()

    def test_empty_substitution_cannot_be_blessed_by_updated_after_hash(self):
        r=deepcopy(self.record);row=r['destination_external_tests']['files'][0]
        (self.root/row['path']).write_bytes(b'');row['sha256']=sha(b'')
        with self.assertRaises(ValueError):self.validate(r)

    def test_wrong_origin_source_original_hash_and_metadata_fail(self):
        for key,value in [('origin_directory','vendor/other'),('source','src/foundation/second/alien.rs'),('source_sha256_before','0'*64),('unknown','value')]:
            with self.subTest(key=key):
                r=deepcopy(self.record);r['destination_external_tests']['files'][0][key]=value
                with self.assertRaises(ValueError):self.validate(r)

    def test_duplicate_missing_wrong_and_escape_routes_fail(self):
        keys=list(self.record['external_file_routes'])
        mutations=[lambda r:r['external_file_routes'].pop(keys[0]),lambda r:r['external_file_routes'][keys[0]].update(destination='../escape'),lambda r:r['external_file_routes'][keys[0]].update(destination=r['external_file_routes'][keys[1]]['destination'])]
        for mutate in mutations:
            r=deepcopy(self.record);mutate(r)
            with self.assertRaises(ValueError):self.validate(r)

    def test_bad_inverse_no_inverse_full_file_and_ambiguity_fail(self):
        key=next(iter(self.record['external_file_routes']))
        for patches in ([],[dict(old=enc(b'wrong::'),new=enc(b'core::'))],[dict(old=enc(b'a'),new=enc((self.root/self.owner['files'][0]['path']).read_bytes()))],[dict(old=enc(b'a'),new=enc(b'\n'))],[dict(old='!!!',new=enc(b'a'))]):
            r=deepcopy(self.record);r['external_file_routes'][key]['replacements']=patches
            with self.assertRaises(ValueError):self.validate(r)

    def test_history_rejects_returned_nonstandard_external_root(self):
        r=deepcopy(self.record)
        r['inputs'][0]['external_tests']['roots']=['test/custom-prior']
        row=r['inputs'][0]['external_tests']['files'][0]
        old=row['path'];row['path']='test/custom-prior/unit.rs'
        r['external_file_routes'][row['path']]=r['external_file_routes'].pop(old)
        self.validate(r,check_current=False)
        (self.root/'test/custom-prior').mkdir()
        with self.assertRaisesRegex(ValueError,'old external transfer root'):
            self.validate(r,check_current=False)

    def test_old_roots_reappearing_wrong_bytes_and_bounds_fail(self):
        p=self.root/'test/foundation/first';p.mkdir()
        with self.assertRaises(ValueError):self.validate()
        p.rmdir()
        r=deepcopy(self.record);r['destination_external_tests']['bytes']+=1
        with self.assertRaises(ValueError):self.validate(r)
        for limit in ('MAX_FILES','MAX_BYTES','MAX_EDITS','MAX_PATCH_BYTES'):
            with self.subTest(limit=limit),patch.object(transfer,limit,1),self.assertRaises(ValueError):self.validate()

    def test_history_keeps_snapshot_but_allows_later_reviewed_current_edits(self):
        self.validate()
        (self.root/self.owner['files'][0]['path']).write_bytes(b'reviewed future test edit\n')
        self.validate(check_current=False)
        with self.assertRaises(ValueError):self.validate()

    def test_loader_accepts_only_exact_grants_and_keeps_row_origins(self):
        mapping=self.root/'compliance/external-tests/fixture.json';mapping.parent.mkdir(parents=True)
        mapping.write_text(json.dumps(dict(schema_version=1,owners=[{k:v for k,v in self.owner.items() if k not in ('origin_directory','file_count','bytes')}])) )
        packages={self.destination:dict(directory=self.destination,origin_directory='vendor/first')}
        with self.assertRaises(ValueError):external_tests.load(self.root,packages)
        owners,_=external_tests.load(self.root,packages,transfers=transfer.grants({'r':self.record}))
        self.assertEqual(owners[self.destination],self.owner)
        data=json.loads(mapping.read_text());data['owners'][0]['files'][1]['origin_directory']='vendor/first';mapping.write_text(json.dumps(data))
        with self.assertRaises(ValueError):external_tests.load(self.root,packages,transfers=transfer.grants({'r':self.record}))

    def test_previous_counter_rejects_lost_or_swapped_origin(self):
        before={self.destination:self.owner};after=deepcopy(before)
        external_tests.check_previous(before,after)
        after[self.destination]['files'][1]['origin_directory']='vendor/first'
        with self.assertRaises(ValueError):external_tests.check_previous(before,after)
        after=deepcopy(before);after[self.destination]['files'].pop()
        with self.assertRaises(ValueError):external_tests.check_previous(before,after)

if __name__ == '__main__':unittest.main()
