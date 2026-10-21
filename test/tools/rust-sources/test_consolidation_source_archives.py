"""Explicit removed Rust implementation archives are not discard/test routes."""
import base64
from copy import deepcopy
import unittest
from unittest.mock import patch
import test_source_consolidation as fixtures
from test_source_consolidation import consolidation, digest


class ArchivedRustSources(unittest.TestCase):
    apply = fixtures.SourceConsolidation.apply
    catalog = fixtures.SourceConsolidation.catalog
    reject = fixtures.SourceConsolidation.reject

    def setUp(self):
        fixtures.SourceConsolidation.setUp(self)
        self.directory = self.old[1]
        self.relative = 'src/output/unused.rs'
        self.raw = b'// Original optional implementation, preserved for review.\npub fn unused() {}\n'
        self.reason = 'Remove the unreferenced optional wrapping algorithm after removing its unsupported public feature; preserve exact prior implementation bytes.'
        self.add_archive(self.relative, self.raw)

    def add_archive(self, relative, raw):
        h = digest(raw)
        self.previous['packages'][self.directory]['files_sha256'][relative] = h
        item = next(i for i in self.item['inputs'] if i['package']['directory'] == self.directory)
        item['package']['files_sha256'][relative] = h
        self.item['file_routes'][self.directory][relative] = dict(kind='archived-source', sha256=h,
            archive_base64=base64.b64encode(raw).decode(), reason=self.reason)

    @property
    def route(self): return self.item['file_routes'][self.directory][self.relative]

    def test_exact_archive_preserves_origin_and_history_not_active_source(self):
        previous = deepcopy(self.previous)
        result = self.apply(); catalog = self.catalog(result)
        self.assertEqual(self.previous, previous)
        self.assertNotIn(self.relative, result[0][self.destination]['files_sha256'])
        archived = next(iter(result[2].values()))['file_routes'][self.directory][self.relative]
        self.assertEqual(base64.b64decode(archived['archive_base64']), self.raw)
        self.assertEqual({p['origin_directory'] for p in [*result[0].values(), *result[1]]},
                         {p['origin_directory'] for p in previous['packages'].values()})
        self.assertEqual(consolidation.validate_history(self.root, catalog), result[2])

    def test_missing_short_or_nontext_reason_rejected(self):
        original=deepcopy(self.route)
        for reason in (None,'','remove',' ' * 60):
            self.route.update(original); self.route['reason']=reason; self.reject('reason')
        self.route.update(original); del self.route['reason']; self.reject()

    def test_changed_bytes_or_declared_hash_rejected(self):
        original=deepcopy(self.route)
        self.route['archive_base64']=base64.b64encode(b'changed').decode(); self.reject('hash')
        self.route.update(original);self.route['sha256']='0'*64;self.reject('hash')

    def test_missing_unknown_or_generic_discard_route_rejected(self):
        original=deepcopy(self.route)
        for field in ('kind','sha256','archive_base64'):
            self.route.clear();self.route.update(original);del self.route[field];self.reject()
        self.route.clear();self.route.update(original);self.route['kind']='discard';self.reject()
        self.route['kind']='archived-source';self.route['destination']='src/buffer.rs';self.reject()

    def test_only_non_entrypoint_rust_implementation_can_be_archived(self):
        for relative in ('Cargo.toml','LICENSE','README.md','compliance/provenance.rs','tests/case.rs',
                         'src/tests/case.rs','src/fixtures/case.rs','src/examples/case.rs','src/lib.rs','src/main.rs','src/data.bin', 'src/tests.rs', 'src/fixtures.rs', 'src/provenance.rs',
                         'src/test.rs', 'src/bench.rs', 'src/benches.rs', 'src/example.rs', 'src/examples.rs',
                         'src/fixture.rs', 'src/legal.rs', 'src/license.rs', 'src/licensing.rs',
                         'src/compliance.rs', 'src/LICENSE.rs', 'src/legal/algorithm.rs',
                         'src/provenance.data.rs', 'src/licenses/license.rs'):
            with self.subTest(relative=relative):
                with self.assertRaises(ValueError):consolidation._source_archive(relative,self.route,digest(self.raw))

    def test_base64_utf8_and_nonempty_bytes_required(self):
        original=deepcopy(self.route)
        for raw in (b'',b'\xff'):
            self.add_archive(self.relative,raw);self.reject()
        self.add_archive(self.relative,self.raw);self.route['archive_base64']='!!!';self.reject()
        self.route.update(original);self.route['archive_base64'] += '\n';self.reject()

    def test_per_file_encoded_and_decoded_bounds(self):
        with patch.object(consolidation,'MAX_ARCHIVE_BYTES',len(self.raw)-1):self.reject('bound')
        self.route['archive_base64']='a'*(4*((consolidation.MAX_ARCHIVE_BYTES+2)//3)+1)
        self.reject('bound')

    def test_aggregate_decoded_bytes_and_count_bounds(self):
        self.add_archive('src/output/another.rs',self.raw)
        with patch.object(consolidation,'MAX_SOURCE_ARCHIVE_TOTAL',len(self.raw)):self.reject('aggregate')
        with patch.object(consolidation,'MAX_SOURCE_ARCHIVES',1):self.reject('aggregate')

    def test_archive_not_allowed_as_external_test_route(self):
        from test_external_transfers import ExactOriginTransfer
        import external_transfers
        fixture=ExactOriginTransfer(); fixture.setUp();self.addCleanup(fixture.doCleanups)
        key=next(iter(fixture.record['external_file_routes']))
        fixture.record['external_file_routes'][key]=deepcopy(self.route)
        with self.assertRaisesRegex(ValueError,'invalid external transfer route'):
            external_transfers.validate(fixture.root,fixture.record)

    def test_archive_does_not_allow_missing_source_route_or_test_owner(self):
        del self.item['file_routes'][self.directory]['src/lib.rs'];self.reject('every before file')
        self.item['file_routes'][self.directory]['src/lib.rs']={'destination':'src/buffer.rs'}
        self.previous['external_tests'][self.directory]={'unrecorded':'owner'}
        self.reject('previous package/external')

    def test_immutable_archive_tampering_rejected(self):
        catalog=self.catalog();record=next(iter(catalog['consolidations'].values()))
        record['file_routes'][self.directory][self.relative]['reason'] += ' tampered'
        with self.assertRaisesRegex(ValueError,'hash drift'):
            consolidation.validate_history(self.root,catalog)


if __name__ == '__main__': unittest.main()
