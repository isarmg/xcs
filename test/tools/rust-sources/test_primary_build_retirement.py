"""Explicit primary static replacement evidence cannot authorize pinned sources."""
import base64
from copy import deepcopy
import unittest
from unittest.mock import patch
import test_source_consolidation as fixtures
from test_source_consolidation import consolidation, digest, inventory


def encoded(raw):
    return base64.b64encode(raw).decode('ascii')


class PrimaryBuildRetirement(unittest.TestCase):
    apply = fixtures.SourceConsolidation.apply
    catalog = fixtures.SourceConsolidation.catalog
    reject = fixtures.SourceConsolidation.reject

    def setUp(self):
        fixtures.SourceConsolidation.setUp(self)
        self.directory = self.old[0]
        self.raw = b'fn main() { println!("cargo:rustc-cfg=old_compiler"); }\n'
        self.route = dict(kind='primary-build-script-retirement', sha256=digest(self.raw),
            archive_base64=encoded(self.raw), reason='Replace compiler probing and generated private exports with reviewed static source for the supported toolchain.',
            original_manifest_base64='', destination_manifest_base64='', replacement_sources={},
            toolchain=dict(rust_version='1.99', targets=['x86_64-unknown-linux-gnu','wasm32-unknown-unknown'],
                           assumptions='The project requires Rust 1.99; removed compiler compatibility probes are no longer needed on the declared native and WASM targets.'))
        self.item['file_routes'][self.directory]['build.rs'] = self.route
        self.source(self.raw)
        self.original(self.manifests[self.directory] + b'build="build.rs"\n')
        self.set_destination(b'[package]\nname="crypto-core"\nversion="1.0.0"\nrust-version="1.99"\nbuild=false\n')
        self.replacement('src/lib.rs')

    def source(self, raw):
        for package in (self.previous['packages'][self.directory], self.item['inputs'][0]['package']):
            package['files_sha256']['build.rs'] = digest(raw)
        self.route.update(sha256=digest(raw), archive_base64=encoded(raw))

    def original(self, raw):
        for package in (self.previous['packages'][self.directory], self.item['inputs'][0]['package']):
            package['files_sha256']['Cargo.toml'] = digest(raw)
        self.route['original_manifest_base64'] = encoded(raw)

    def set_destination(self, raw):
        (self.root / self.destination_path / 'Cargo.toml').write_bytes(raw)
        self.item['destination_files_sha256'] = inventory(self.root / self.destination_path)
        self.route['destination_manifest_base64'] = encoded(raw)

    @property
    def destination_path(self):
        return self.item['destination_directory']

    def replacement(self, path, raw=None):
        if raw is None:
            raw=(self.root / self.destination_path / path).read_bytes()
        self.route['replacement_sources'][path] = dict(sha256=digest(raw), archive_base64=encoded(raw))

    def test_primary_retirement_preserves_exact_immutable_evidence(self):
        before = deepcopy(self.plan)
        catalog = self.catalog()
        self.assertEqual(before, self.plan)
        consolidation.validate_history(self.root, catalog)
        stored=next(iter(catalog['consolidations'].values()))['file_routes'][self.directory]['build.rs']
        self.assertEqual(base64.b64decode(stored['archive_base64']), self.raw)
        stored['toolchain']['assumptions'] += ' changed'
        with self.assertRaisesRegex(ValueError,'hash drift'):
            consolidation.validate_history(self.root,catalog)

    def test_secondary_kind_still_rejects_primary(self):
        self.route['kind']='archived-build-script'
        for key in ('toolchain','replacement_sources','original_manifest_base64'):del self.route[key]
        self.reject('secondary root')

    def test_primary_kind_rejects_secondary_and_nonroot_paths(self):
        for directory,path in ((self.old[1],'build.rs'),(self.directory,'src/build.rs'),(self.directory,'LICENSE')):
            with self.assertRaisesRegex(ValueError,'primary root'):
                consolidation._primary_build_retirement(self.root,path,self.route,digest(self.raw),directory,
                                                        self.item['inputs'][0]['package'],self.item)

    def test_original_identity_build_and_legal_guards(self):
        original=base64.b64decode(self.route['original_manifest_base64'])
        for raw in (original.replace(b'digest',b'wrong'),original.replace(b'1.0.0',b'9.0.0'),
                    original.replace(b'"build.rs"',b'false'),original.replace(b'"build.rs"',b'"custom.rs"'),
                    original+b'links="native"\n',original+b'license-file="src/../build.rs"\n'):
            self.original(raw);self.reject('identity/build/legal')
        self.original(original)
        self.item['inputs'][0]['package']['license_materials']=[self.directory+'/build.rs']
        self.reject('legal')

    def test_exact_original_and_destination_manifests_required(self):
        for field in ('original_manifest_base64','destination_manifest_base64'):
            old=self.route[field];self.route[field]=encoded(b'[package]\nname="wrong"\n')
            self.reject('before hash');self.route[field]=old

    def test_inert_destination_and_matching_version_required(self):
        original=base64.b64decode(self.route['destination_manifest_base64'])
        for raw in (original.replace(b'build=false\n',b''),original.replace(b'false',b'"custom.rs"'),
                    original+b'links="native"\n',original.replace(b'1.0.0',b'9.0.0')):
            self.set_destination(raw);self.reject('inert')

    def test_source_reintroduction_rejected(self):
        for path in ('build.rs','src/copy.rs'):
            self.item['destination_files_sha256'][path]=digest(self.raw)
            self.reject('inert');del self.item['destination_files_sha256'][path]

    def test_primary_manifest_route_must_be_preserved(self):
        self.item['file_routes'][self.directory]['Cargo.toml']={'destination':'src/lib.rs'}
        self.reject('manifest route')

    def test_explicit_toolchain_assumptions_required(self):
        original=deepcopy(self.route['toolchain'])
        for field,value in (('rust_version','1.56'),('targets',['aarch64-unknown-linux-gnu']),
                            ('assumptions','short'),('extra','unrecognized')):
            self.route['toolchain']=dict(original,**{field:value});self.reject('toolchain')
        self.route['toolchain']=original
        self.set_destination(base64.b64decode(self.route['destination_manifest_base64']).replace(b'1.99',b'1.56'))
        self.reject('toolchain')

    def test_missing_entrypoint_or_excess_replacements_rejected(self):
        self.route['replacement_sources']={};self.reject('bounded static')
        self.route['replacement_sources']={'src/lib.rs':{},**{f'src/a{i}.rs':{} for i in range(16)}}
        self.reject('bounded static')

    def test_only_inventoried_production_replacements_accepted(self):
        for path in ('../src/x.rs','src/main.rs','src/build.rs','src/tests.rs','src/legal/x.rs','src/fixtures/x.rs','Cargo.toml','src/missing.rs'):
            self.replacement(path,b'fn replacement() {}')
            self.reject();del self.route['replacement_sources'][path]

    def test_exact_replacement_bytes_and_inventory_hash_required(self):
        self.replacement('src/lib.rs',b'changed');self.reject('hash')
        self.replacement('src/lib.rs');self.route['replacement_sources']['src/lib.rs']['extra']='no'
        self.reject('production source')

    def test_nonutf8_replacement_rejected_even_with_matching_inventory(self):
        self.replacement('src/lib.rs',b'\xff');self.item['destination_files_sha256']['src/lib.rs']=digest(b'\xff')
        self.reject('UTF-8')

    def test_replacement_file_and_total_bounds(self):
        raw=b'x'*(256*1024+1);self.replacement('src/lib.rs',raw)
        self.item['destination_files_sha256']['src/lib.rs']=digest(raw);self.reject('byte bound')
        self.route['replacement_sources']={}
        for i in range(5):
            path='src/lib.rs' if i==0 else f'src/static{i}.rs';raw=b'x'*(256*1024)
            self.replacement(path,raw);self.item['destination_files_sha256'][path]=digest(raw)
        self.reject('byte bound')

    def test_original_source_guards_and_aggregate_bounds_preserved(self):
        for raw in (b'',b'\xff'):
            self.source(raw);self.reject()
        self.source(self.raw)
        with patch.object(consolidation,'MAX_SOURCE_ARCHIVES',0):self.reject('aggregate')
        self.route['reason']='short';self.reject('reason')

    def test_missing_or_extra_disposition_fields_rejected(self):
        original=deepcopy(self.route)
        for field in original:
            self.route.clear();self.route.update(original);del self.route[field];self.reject()
        self.route.clear();self.route.update(original);self.route['destination']='src/lib.rs';self.reject()

    def test_current_destination_cannot_reenable_build(self):
        catalog=self.catalog()
        (self.root/self.destination_path/'Cargo.toml').write_text('[package]\nname="crypto-core"\nversion="1.0.0"\nbuild=true\n')
        with self.assertRaisesRegex(ValueError,'inert'):consolidation.validate_history(self.root,catalog)

    def test_pinned_authorizer_cannot_override_primary_retirement_guard(self):
        proof=self.root/'compliance/platform-test-relocation.json'
        proof.parent.mkdir(parents=True)
        proof.write_text(self.directory)
        with self.assertRaisesRegex(ValueError,'cannot authorize pinned'):
            consolidation.prepare(self.root,self.previous,self.plan,inventory,authorize_pinned=lambda *args:True)

    def test_prior_history_cannot_shadow_the_retirement_being_reviewed(self):
        proof=self.root/'compliance/platform-test-relocation.json'
        proof.parent.mkdir(parents=True)
        proof.write_text(self.directory)
        history={'older':{'inputs':[], 'destination_directory':'src/foundation/unrelated', 'file_routes':{}}}
        inputs={row['package']['directory']:row['package'] for row in self.item['inputs']}
        with self.assertRaisesRegex(ValueError,'cannot authorize pinned'):
            consolidation._unhandled_history(self.root,self.previous,inputs,self.destination_path,
                                             history,self.item,lambda *args:True)
    def test_pinned_projection_still_refuses_archived_primary(self):
        import proof_consolidation
        with patch.object(proof_consolidation,'_reject_pinned_secondary'),patch.object(proof_consolidation.external_transfers,'validate'):
            with self.assertRaisesRegex(ValueError,'pinned primary files cannot be archived'):
                proof_consolidation.projection(self.root,self.item)


if __name__=='__main__':unittest.main()
