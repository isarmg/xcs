from copy import deepcopy
import hashlib
from pathlib import Path
import tempfile
import unittest
import sys
sys.path.insert(0, str(Path(__file__).resolve().parents[3] / 'tools/rust-sources'))
import source_moves as moves


def digest(data):
    return hashlib.sha256(data).hexdigest()


def inventory(root):
    result = {}
    for p in root.rglob('*'):
        if p.is_symlink():
            raise ValueError('symlink')
        if p.is_file():
            result[p.relative_to(root).as_posix()] = digest(p.read_bytes())
    return result


class InternalMoves(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.directory = 'src/foundation/flag-set'
        self.path = self.root / self.directory
        self.path.mkdir(parents=True)
        (self.path / 'Cargo.toml').write_text('[package]\nname="flag-set"\nversion="1.0.0"\n')
        (self.path / 'serialization.rs').write_text('new content')
        after = inventory(self.path)
        before = dict(after)
        before['serde.rs'] = digest(b'old content')
        del before['serialization.rs']
        self.package = dict(directory=self.directory, name='flag-set', version='1.0.0',
                            origin_directory='vendor/bitflags-1.0.0', upstream={'name':'bitflags','version':'1.0.0'}, files_sha256=before)
        self.previous = dict(schema=5, packages={self.directory:self.package}, external_tests={}, merges=[])
        self.item = dict(directory=self.directory, package=deepcopy(self.package), merged_packages=[],
                         files_sha256=after, file_moves={'serde.rs':'serialization.rs'}, reason='Functional internal names',
                         external_tests=None, destination_external_tests=None, external_file_moves={})
        self.plan = dict(schema=1, packages=[self.item])

    def apply(self):
        return moves.prepare(self.root,self.previous,self.plan,inventory)

    def reject(self, pattern='.'):
        with self.assertRaisesRegex(ValueError,pattern): self.apply()

    def add_external(self):
        base='test/foundation/flag-set'
        (self.root/base).mkdir(parents=True)
        (self.root/base/'serialization.rs').write_text('test after')
        owner=dict(directory=self.directory,origin_directory=self.package['origin_directory'],roots=[base],file_count=1,bytes=11,
                   files=[dict(source='src/foundation/bitflags-1.0.0/tests/serde.rs',source_sha256_before=digest(b'test before'),
                               path=base+'/serde.rs',sha256=digest(b'test before'))])
        after=deepcopy(owner);after['bytes']=10
        after['files'][0].update(path=base+'/serialization.rs',sha256=digest(b'test after'))
        self.previous['external_tests'][self.directory]=owner
        self.item.update(external_tests=deepcopy(owner),destination_external_tests=after,external_file_moves={base+'/serde.rs':base+'/serialization.rs'})

    def test_exact_move_preserves_original_receipt(self):
        original=deepcopy(self.previous)
        active,history,changed=self.apply()
        self.assertEqual(original,self.previous)
        self.assertEqual(active[self.directory]['files_sha256'],self.item['files_sha256'])
        self.assertEqual(next(iter(history.values()))['package'],self.package)
        normalized=moves.remap_previous(self.previous['packages'],changed)
        self.assertEqual(normalized[self.directory]['files_sha256']['serialization.rs'],self.package['files_sha256']['serde.rs'])
        moves.validate_history(self.root,dict(schema=6,source_moves=history))
        moves.validate_origins(history,{self.package['origin_directory']:{'upstream':self.package['upstream']}})

    def test_no_plan_does_not_learn_move(self):
        active,history,changed=moves.prepare(self.root,self.previous,None,inventory)
        self.assertEqual(active,self.previous['packages']);self.assertEqual(history,{});self.assertEqual(changed,{})

    def test_external_move_preserves_historical_labels(self):
        self.add_external();_,history,changed=self.apply()
        moves.check_current_external({self.directory:self.item['destination_external_tests']},changed)
        self.assertEqual(next(iter(history.values()))['external_tests'],self.previous['external_tests'][self.directory])

    def test_external_only_move(self):
        self.add_external();self.package['files_sha256']=inventory(self.path)
        self.item['package']=deepcopy(self.package);self.item['file_moves']={}
        self.apply()

    def test_same_identity_second_move_preserves_both_receipts(self):
        active,history,_=self.apply()
        old=deepcopy(history)
        self.previous=dict(self.previous,schema=6,packages=active,source_moves=history)
        (self.path/'serialization.rs').rename(self.path/'codec.rs')
        self.item.update(package=deepcopy(active[self.directory]),files_sha256=inventory(self.path),file_moves={'serialization.rs':'codec.rs'})
        _,new,_=self.apply()
        self.assertEqual(len(new),2)
        for key,value in old.items():self.assertEqual(new[key],value)

    def test_history_tamper_rejected(self):
        _,history,_=self.apply();next(iter(history.values()))['reason']='changed'
        with self.assertRaisesRegex(ValueError,'receipt drift'):moves.validate_history(self.root,dict(schema=6,source_moves=history))

    def test_history_schema_required(self):
        _,history,_=self.apply()
        with self.assertRaisesRegex(ValueError,'schema 6'):moves.validate_history(self.root,dict(schema=5,source_moves=history))

    def test_origin_mismatch_rejected(self):
        _,history,_=self.apply()
        with self.assertRaisesRegex(ValueError,'origin mismatch'):moves.validate_origins(history,{})

    def test_stale_before_rejected(self):
        self.item['package']['name']='other';self.reject('exact prior')

    def test_content_not_reviewed_rejected(self):
        (self.path/'serialization.rs').write_text('unexpected');self.reject('content differs')

    def test_version_change_rejected(self):
        p=self.path/'Cargo.toml';p.write_text(p.read_text().replace('1.0.0','2.0.0'))
        self.item['files_sha256']=inventory(self.path);self.reject('name/version')

    def test_unknown_path_rejected(self):
        self.item['file_moves']={'unknown':'serialization.rs'};self.reject('exact inventory')

    def test_overwrite_unmoved_rejected(self):
        self.item['file_moves']={'serde.rs':'Cargo.toml'};self.reject('bijective')

    def test_empty_moves_rejected(self):
        self.item['file_moves']={};self.reject('identity/reason/map')

    def test_duplicate_plan_rejected(self):
        self.plan['packages'].append(deepcopy(self.item));self.reject('duplicate')

    def test_loss_rejected(self):
        del self.item['files_sha256']['Cargo.toml'];self.reject('bijective')

    def test_exact_overlapping_origin_receipts_required(self):
        merged=deepcopy(self.package);merged.update(origin_directory='vendor/other-1',upstream={'name':'other','version':'1'})
        self.previous['merges']=[merged];self.reject('exact prior')
        self.item['merged_packages']=[deepcopy(merged)];self.apply()

    def test_external_owner_may_belong_to_exact_merged_origin(self):
        self.add_external()
        merged=deepcopy(self.package);merged.update(origin_directory='vendor/other-1',upstream={'name':'other','version':'1'})
        self.previous['merges']=[merged];self.item['merged_packages']=[deepcopy(merged)]
        self.previous['external_tests'][self.directory]['origin_directory']=merged['origin_directory']
        self.item['external_tests']['origin_directory']=merged['origin_directory']
        self.item['destination_external_tests']['origin_directory']=merged['origin_directory']
        self.apply()

    def test_integrated_package_supported(self):
        old=self.directory;self.directory='tools/code-helper'
        new=self.root/self.directory;new.parent.mkdir(parents=True);self.path.rename(new);self.path=new
        self.package['directory']=self.directory
        self.previous['packages']={self.directory:self.package}
        self.item.update(directory=self.directory,package=deepcopy(self.package));self.apply()

    def test_external_original_source_cannot_change(self):
        self.add_external();self.item['destination_external_tests']['files'][0]['source']=self.directory+'/tests/new.rs'
        self.reject('rewrote historical')

    def test_external_content_not_reviewed_rejected(self):
        self.add_external();(self.root/'test/foundation/flag-set/serialization.rs').write_text('unexpected')
        self.reject('external content differs')

    def test_external_destination_cannot_escape_root(self):
        self.add_external();old=next(iter(self.item['external_file_moves']))
        self.item['external_file_moves'][old]='test/unowned.rs'
        self.item['destination_external_tests']['files'][0]['path']='test/unowned.rs'
        self.reject('escaped owned roots')

    def test_external_owner_cannot_disappear(self):
        self.add_external();self.item['destination_external_tests']=None;self.reject('external owner')

    def test_external_roots_cannot_change(self):
        self.add_external();self.item['destination_external_tests']['roots']=['test/other'];self.reject('identity or roots')

    def test_old_external_file_must_not_remain(self):
        self.add_external();(self.root/'test/foundation/flag-set/serde.rs').write_text('test before');self.reject('external content differs')


if __name__=='__main__':unittest.main()
