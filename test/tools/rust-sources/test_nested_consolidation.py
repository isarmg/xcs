"""Second-stage physical ownership retains an immutable, exact provenance DAG."""
import base64
from copy import deepcopy
import json
from pathlib import Path
import shutil
import sys
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / 'tools/rust-sources'))
import external_tests
import external_transfers
import source_consolidation as c
import source_consolidation_chain as chain
import test_source_consolidation as fixture
from test_source_consolidation import digest, inventory


class NestedConsolidation(unittest.TestCase):
    def setUp(self):
        fixture.SourceConsolidation.setUp(self)
        self.counter = 0

    def apply(self):
        return c.prepare(self.root, self.previous, self.plan, inventory,
                         validate_external=external_transfers.validate)

    def catalog(self):
        active, merges, records, changed = self.apply()
        external = c.remap_previous_external(self.previous.get('external_tests', {}), changed)
        return dict(self.previous, schema=8, packages=active, merges=merges,
                    external_tests=external, consolidations=records,
                    consolidation_records_sha256=c.hashes(records))

    def install_tests(self):
        after = dict(directory=self.destination, origin_directory=self.item['inputs'][0]['package']['origin_directory'],
                     roots=['test/foundation/crypto-core'], files=[], file_count=3, bytes=0)
        for index, item in enumerate(self.item['inputs']):
            package = item['package']; directory = package['directory']
            old_root = 'test/foundation/' + directory.rsplit('/', 1)[1]
            data = ('// fragment %s\nassert!(true);\n' % index).encode()
            old = dict(path=old_root + '/unit.rs', source=directory + '/src/lib.rs',
                       source_sha256_before=package['files_sha256']['src/lib.rs'], sha256=digest(data))
            owner = dict(directory=directory, origin_directory=package['origin_directory'],
                         roots=[old_root], files=[old], file_count=1, bytes=len(data))
            item['external_tests'] = deepcopy(owner)
            self.previous['external_tests'][directory] = owner
            target = after['roots'][0] + '/unit-%s.rs' % index
            path = self.root / target; path.parent.mkdir(parents=True, exist_ok=True); path.write_bytes(data)
            after['files'].append(dict(old, path=target, origin_directory=package['origin_directory']))
            after['bytes'] += len(data)
            self.item['external_file_routes'][old['path']] = dict(destination=target, replacements=[])
        self.item['destination_external_tests'] = after

    def stage(self, secondary=False, move=False):
        self.previous = self.catalog()
        self.counter += 1
        old = self.destination
        old_package = deepcopy(self.previous['packages'][old])
        old_manifest = (self.root / old / 'Cargo.toml').read_bytes()
        new_directory = 'src/foundation/extra-' + str(self.counter)
        new_name = new_directory.rsplit('/', 1)[1]
        raw = ('[package]\nname="%s"\nversion="2.0.0"\n' % new_name).encode()
        added = dict(directory=new_directory, name=new_name, version='2.0.0',
                     origin_directory='vendor/' + new_name, upstream=dict(name=new_name, version='2.0.0'),
                     files_sha256={'Cargo.toml': digest(raw), 'src/lib.rs': digest(b'pub fn extra() {}\n')})
        self.previous['packages'][new_directory] = deepcopy(added)
        destination = 'src/foundation/next-core-' + str(self.counter) if move or secondary else old
        if destination != old:
            (self.root / old).rename(self.root / destination)
        path = self.root / destination
        name = destination.rsplit('/', 1)[1]
        version = added['version'] if secondary else old_package['version']
        (path / 'Cargo.toml').write_text('[package]\nname="%s"\nversion="%s"\n' % (name, version))
        added_path = 'src/extra-%s.rs' % self.counter
        (path / added_path).write_bytes(b'pub fn extra() {}\n')
        old_external = deepcopy(self.previous['external_tests'].get(old))
        external = deepcopy(old_external)
        routes = {}
        if external:
            external['directory'] = destination
            external['origin_directory'] = added['origin_directory'] if secondary else old_package['origin_directory']
            if destination != old:
                root = 'test/foundation/' + name
                (self.root / external['roots'][0]).rename(self.root / root)
                previous_root = external['roots'][0]; external['roots'] = [root]
                for row in external['files']:
                    before = row['path']; row['path'] = root + row['path'][len(previous_root):]
                    routes[before] = dict(destination=row['path'], replacements=[])
            else:
                routes = {row['path']: dict(destination=row['path'], replacements=[]) for row in external['files']}
        inputs = [dict(package=old_package, external_tests=old_external), dict(package=added, external_tests=None)]
        self.item = dict(primary_directory=new_directory if secondary else old,
            destination_directory=destination, destination_name=name, reason='Reviewed real second-stage ownership consolidation',
            inputs=inputs, destination_files_sha256=inventory(path),
            file_routes={old: {relative: {'destination': relative} for relative in old_package['files_sha256']},
                         new_directory: {'Cargo.toml': {'archive_base64': base64.b64encode(raw).decode()},
                                         'src/lib.rs': {'destination': added_path}}},
            new_files={}, destination_external_tests=external, external_file_routes=routes)
        if secondary:
            self.item['file_routes'][old]['Cargo.toml'] = {'archive_base64': base64.b64encode(old_manifest).decode()}
            self.item['file_routes'][new_directory]['Cargo.toml'] = {'destination': 'Cargo.toml'}
        _, consumed, _ = chain.analyze(self.root, self.previous['consolidations'])
        self.item['input_consolidations'] = chain.expected(self.previous,
            {i['package']['directory']: i['package'] for i in inputs}, self.previous['consolidations'], consumed)
        self.plan = dict(schema=1, packages=[self.item])
        self.destination = destination
        return old

    def verify(self, catalog):
        return c.validate_history(self.root, catalog, validate_external=external_transfers.validate)

    def reject(self, message='.'):
        with self.assertRaisesRegex(ValueError, message):
            self.apply()

    def bind(self):
        import source_history_chain
        self.item['input_history'] = source_history_chain.expected(self.previous,
            {i['package']['directory']: i['package'] for i in self.item['inputs']}, self.destination)

    def test_nested_replay_keeps_original_rename_binding(self):
        import test_source_history_chain
        import source_naming
        test_source_history_chain.SourceHistoryChain.rename(self)
        self.stage(secondary=True)
        catalog = self.catalog()
        self.assertEqual(source_naming.validate_history(self.root, catalog), self.previous['renames'])
        self.verify(catalog)

    def test_rehashed_duplicate_nested_origin_rejected(self):
        self.stage(); catalog = self.catalog()
        record = catalog['consolidations'].pop(c.receipt_id(self.item))
        merged = next(iter(record['input_consolidations'].values()))['merged_packages']
        merged.append(deepcopy(merged[0]))
        catalog['consolidations'][c.receipt_id(record)] = record
        catalog['consolidation_records_sha256'] = c.hashes(catalog['consolidations'])
        with self.assertRaisesRegex(ValueError, 'duplicate origin'):
            self.verify(catalog)

    def test_pinned_nested_receipt_requires_new_adapter(self):
        self.stage()
        predecessor = next(iter(self.item['input_consolidations'].values()))['receipt_id']
        prior = self.previous['consolidations'][predecessor]
        self.previous['proof_routing_receipts'] = ['0' * 64]
        import proof_routing
        with patch.object(proof_routing, '_read', return_value={'binding': {'kind': 'consolidation', 'record': prior}}):
            self.reject('nested pinned source')

    def test_benchmark_pinned_nested_owner_stays_rejected(self):
        self.stage()
        import benchmark_relocation
        self.previous[benchmark_relocation.KEY] = benchmark_relocation.RECEIPT
        with patch.object(benchmark_relocation, 'SCOPES', {self.destination: ('benches', 1)}):
            self.reject('benchmark-pinned')

    def test_two_multi_origin_inputs_join_without_flattening_receipts(self):
        self.previous = self.catalog()
        crypto = self.destination
        other = fixture.SourceConsolidation.additional_group(self)
        self.item, self.plan = other, dict(schema=1, packages=[other])
        self.previous = self.catalog()
        cli = other['destination_directory']
        old_records = deepcopy(self.previous['consolidations'])
        cli_manifest = (self.root / cli / 'Cargo.toml').read_bytes()
        (self.root / crypto / 'src/cli.rs').write_bytes((self.root / cli / 'src/lib.rs').read_bytes())
        shutil.rmtree(self.root / cli)
        inputs = [dict(package=deepcopy(self.previous['packages'][directory]), external_tests=None)
                  for directory in (crypto, cli)]
        self.item = dict(primary_directory=crypto, destination_directory=crypto,
            destination_name='crypto-core', reason='Combine two independently proven physical owners', inputs=inputs,
            destination_files_sha256=inventory(self.root / crypto),
            file_routes={crypto: {path: dict(destination=path) for path in inputs[0]['package']['files_sha256']},
                         cli: {'Cargo.toml': dict(archive_base64=base64.b64encode(cli_manifest).decode()),
                               'src/lib.rs': dict(destination='src/cli.rs')}},
            new_files={}, destination_external_tests=None, external_file_routes={})
        _, consumed, _ = chain.analyze(self.root, old_records)
        self.item['input_consolidations'] = chain.expected(self.previous,
            {i['package']['directory']: i['package'] for i in inputs}, old_records, consumed)
        self.plan = dict(schema=1, packages=[self.item])
        catalog = self.catalog(); self.verify(catalog)
        self.assertEqual(len(catalog['merges']), 4)
        self.assertEqual({key: catalog['consolidations'][key] for key in old_records}, old_records)

    def test_cycle_is_rejected_before_projection(self):
        first, second = deepcopy(self.item), deepcopy(self.item)
        for index, record in enumerate((first, second)):
            for item in record['inputs'][1:]:
                item['package']['directory'] += '-cycle-' + str(index)
                item['package']['origin_directory'] += '-cycle-' + str(index)
            directory = record['inputs'][0]['package']['directory']
            record['input_consolidations'] = {directory: dict(receipt_id='b' if index == 0 else 'a',
                merged_packages=[deepcopy(record['inputs'][1]['package'])])}
        with self.assertRaisesRegex(ValueError, 'cyclic'):
            chain.analyze(self.root, {'a': first, 'b': second})

    def test_primary_same_root_keeps_receipts_bytes_and_every_origin(self):
        self.stage(); before = deepcopy(self.previous)
        encoded = {key: c._encoded(value) for key, value in before['consolidations'].items()}
        catalog = self.catalog()
        self.assertEqual(len(catalog['packages']), 1); self.assertEqual(len(catalog['merges']), 3)
        for key, data in encoded.items():
            self.assertEqual(c._encoded(catalog['consolidations'][key]), data)
        self.assertEqual(set(c._origin_rows(catalog)), set(c._origin_rows(before)))
        self.assertEqual(self.previous, before)
        self.verify(catalog)

    def test_secondary_nested_owner_and_nested_replay(self):
        self.install_tests(); self.stage(secondary=True)
        first = self.catalog(); self.verify(first)
        history = deepcopy(first['consolidations'])
        # A further stage must consume the newest output, not its ancestor.
        self.stage(move=True)
        final = self.catalog(); self.verify(final)
        self.assertEqual(len(final['consolidations']), 3)
        self.assertEqual(len(final['merges']), 4)
        self.assertEqual({key: final['consolidations'][key] for key in history}, history)
        active, merged, retained, changed = c.prepare(self.root, final, None, inventory,
                                                    validate_external=external_transfers.validate)
        self.assertEqual((active, merged, retained, changed), (final['packages'], final['merges'], final['consolidations'], {}))

    def test_nested_external_loader_uses_only_exact_inherited_grants(self):
        self.install_tests(); self.stage(secondary=True); catalog = self.catalog()
        owner = catalog['external_tests'][self.destination]
        mapping = self.root / 'compliance/external-tests/nested.json'; mapping.parent.mkdir(parents=True)
        mapping.write_text(json.dumps(dict(schema_version=1, owners=[{key: value for key, value in owner.items()
            if key not in ('origin_directory', 'file_count', 'bytes')}])) )
        current, _ = external_tests.load(self.root, catalog['packages'], transfers=external_transfers.grants(catalog['consolidations']))
        self.assertEqual(current[self.destination], owner)
        with self.assertRaisesRegex(ValueError, 'ownership'):
            external_tests.load(self.root, catalog['packages'])

    def test_missing_duplicate_wrong_or_stale_merged_snapshots(self):
        self.stage()
        original = deepcopy(self.item)
        for mutation in (
            lambda b: b['merged_packages'].pop(),
            lambda b: b['merged_packages'].append(deepcopy(b['merged_packages'][0])),
            lambda b: b['merged_packages'][0].update(origin_directory='vendor/wrong'),
            lambda b: b['merged_packages'][0]['files_sha256'].update({'src/lib.rs': '0' * 64}),
            lambda b: b.update(receipt_id='0' * 64)):
            with self.subTest(mutation=mutation):
                self.item.clear(); self.item.update(deepcopy(original))
                mutation(next(iter(self.item['input_consolidations'].values())))
                self.reject('exact previous')
        self.item.clear(); self.item.update(original); self.item.pop('input_consolidations')
        self.reject('chain-aware')

    def test_stale_primary_package_snapshot(self):
        self.stage(); self.item['inputs'][0]['package']['files_sha256']['src/lib.rs'] = '0' * 64
        self.reject('exact previous')

    def test_wrong_external_origin_source_or_hash_rejected(self):
        self.install_tests(); self.stage()
        original = deepcopy(self.item)
        for key, value in [('origin_directory', 'vendor/alien'), ('source', 'src/foundation/alien/lib.rs'),
                           ('source_sha256_before', '0' * 64)]:
            self.item.clear(); self.item.update(deepcopy(original))
            self.item['destination_external_tests']['files'][1][key] = value
            self.reject('historical attribution')

    def test_wrong_nested_external_inverse_rejected(self):
        self.install_tests(); self.stage()
        row = self.item['destination_external_tests']['files'][1]
        path = self.root / row['path']; data = path.read_bytes().replace(b'true', b'false'); path.write_bytes(data)
        row['sha256'] = digest(data); self.item['destination_external_tests']['bytes'] += 1
        self.reject('without inverse proof')

    def test_direct_nested_external_validation_requires_authenticated_grants(self):
        self.install_tests(); self.stage()
        with self.assertRaisesRegex(ValueError, 'authenticated predecessor grants'):
            external_transfers.validate(self.root, self.item)

    def test_missing_ancestor_receipt_rejected(self):
        self.stage(); catalog = self.catalog()
        prior = next(iter(self.item['input_consolidations'].values()))['receipt_id']
        del catalog['consolidations'][prior]
        catalog['consolidation_records_sha256'] = c.hashes(catalog['consolidations'])
        with self.assertRaisesRegex(ValueError, 'exact predecessor'):
            self.verify(catalog)

    def test_ancestor_tampering_is_not_authorized_by_descendant(self):
        self.stage(); catalog = self.catalog()
        next(iter(catalog['consolidations'].values()))['reason'] += ' tampered'
        with self.assertRaisesRegex(ValueError, 'hash drift'):
            self.verify(catalog)

    def test_rehashed_nested_missing_origin_rejected(self):
        self.stage(); catalog = self.catalog()
        key = c.receipt_id(self.item); record = catalog['consolidations'].pop(key)
        next(iter(record['input_consolidations'].values()))['merged_packages'].pop()
        catalog['consolidations'][c.receipt_id(record)] = record
        catalog['consolidation_records_sha256'] = c.hashes(catalog['consolidations'])
        with self.assertRaisesRegex(ValueError, 'origin coverage'):
            self.verify(catalog)

    def test_replay_old_parent_rejected(self):
        self.stage(); self.stage()
        newest = next(iter(self.item['input_consolidations'].values()))
        old = next(record for record in self.previous['consolidations'].values() if record.get('input_consolidations'))
        newest['receipt_id'] = next(iter(old['input_consolidations'].values()))['receipt_id']
        self.reject('exact previous')

    def test_branching_predecessor_rejected(self):
        self.stage(); catalog = self.catalog()
        duplicate = deepcopy(self.item); duplicate['reason'] += ' forged fork'
        catalog['consolidations'][c.receipt_id(duplicate)] = duplicate
        catalog['consolidation_records_sha256'] = c.hashes(catalog['consolidations'])
        with self.assertRaisesRegex(ValueError, 'duplicate'):
            self.verify(catalog)

    def test_rechecks_current_manifest_after_prior_success(self):
        self.stage(secondary=True); catalog = self.catalog(); self.verify(catalog)
        (self.root / self.destination / 'Cargo.toml').write_text('[package]\nname="wrong"\nversion="2.0.0"\n')
        with self.assertRaisesRegex(ValueError, 'manifest name/version'):
            self.verify(catalog)

    def test_current_merged_inventory_and_identity_stay_exact(self):
        self.stage(); catalog = self.catalog()
        for mutate in (lambda row: row.update(origin_directory='vendor/lost'),
                       lambda row: row.update(license='wrong'),
                       lambda row: row['files_sha256'].update({'src/lib.rs': '0' * 64})):
            changed = deepcopy(catalog); mutate(changed['merges'][0])
            with self.assertRaisesRegex(ValueError, 'origin coverage|shared inventory'):
                self.verify(changed)

    def test_current_external_loss_or_origin_swap_is_rejected(self):
        self.install_tests(); self.stage(); catalog = self.catalog()
        for mutation in (lambda owner: owner['files'].pop(),
                         lambda owner: owner['files'][1].update(origin_directory='vendor/forged')):
            changed = deepcopy(catalog); mutation(changed['external_tests'][self.destination])
            with self.assertRaisesRegex(ValueError, 'current external origin accounting shrank'):
                self.verify(changed)

    def test_nested_depth_bound(self):
        self.stage()
        with patch.object(chain, 'MAX_DEPTH', 1):
            self.reject('depth bound')

    def test_new_native_tests_after_prior_receipt_are_accounted(self):
        self.install_tests(); self.stage()
        owner = self.previous['external_tests'][self.item['primary_directory']]
        data = b'// later native test\nassert!(true);\n'
        row = dict(path=owner['roots'][0] + '/later.rs', source=self.item['primary_directory'] + '/src/lib.rs',
                   source_sha256_before=digest(b'later source'), sha256=digest(data))
        (self.root / row['path']).write_bytes(data)
        owner['files'].append(row); owner['file_count'] += 1; owner['bytes'] += len(data)
        self.item['inputs'][0]['external_tests'] = deepcopy(owner)
        after = self.item['destination_external_tests']
        after['files'].append(dict(row, origin_directory=owner['origin_directory']))
        after['file_count'] += 1; after['bytes'] += len(data)
        self.item['external_file_routes'][row['path']] = dict(destination=row['path'], replacements=[])
        self.verify(self.catalog())

# Manager integration uses synthetic origins/metadata, not Cargo subprocesses.
import test_manage_consolidation as manager_fixture


class NestedManager(unittest.TestCase):
    setUp = manager_fixture.ManageConsolidation.setUp
    origin = manager_fixture.ManageConsolidation.origin
    write_json = manager_fixture.ManageConsolidation.write_json
    save_catalog = manager_fixture.ManageConsolidation.save_catalog
    refresh = manager_fixture.ManageConsolidation.refresh
    verify = manager_fixture.ManageConsolidation.verify
    _workspace = manager_fixture.ManageConsolidation._workspace
    _install_external_baseline = manager_fixture.ManageConsolidation._install_external_baseline
    _write_after_mapping = manager_fixture.ManageConsolidation._write_after_mapping
    stage = manager_fixture.ManageConsolidation.stage
    commit = manager_fixture.ManageConsolidation.commit

    def nested(self, secondary):
        manager = manager_fixture.manager
        self.commit(external=True)
        old = self.destination
        extra = 'src/foundation/extra'
        path = self.root / extra; path.mkdir()
        raw = b'[package]\nname="extra"\nversion="3.0.0"\n'
        (path / 'Cargo.toml').write_bytes(raw)
        (path / 'lib.rs').write_text('pub fn extra() {}\n')
        upstream = dict(name='extra', version='3.0.0', archive_sha256='e' * 64,
                        archive_url='https://static.crates.io/crates/extra/extra-3.0.0.crate')
        self.origin_records.append(self.origin('old/extra', upstream))
        self.write_json(manager.ORIGINS, dict(packages=self.origin_records))
        added = deepcopy(self.catalog['packages'][old])
        added.update(name='extra', version='3.0.0', directory=extra, origin_directory='old/extra',
                     upstream=upstream, files_sha256=manager.files(path))
        self.catalog['packages'][extra] = added
        self._workspace([(old, 'combined-core', '1.0.0'), (extra, 'extra', '3.0.0')])
        self.save_catalog(); self.refresh(); self.verify()
        before = deepcopy(self.catalog)
        destination = 'src/foundation/final-core' if secondary else old
        old_manifest = (self.root / old / 'Cargo.toml').read_bytes()
        if destination != old:
            (self.root / old).rename(self.root / destination)
        path = self.root / destination
        version = '3.0.0' if secondary else '1.0.0'
        name = destination.rsplit('/', 1)[1]
        (path / 'Cargo.toml').write_text('[package]\nname="%s"\nversion="%s"\n' % (name, version))
        (path / 'extra.rs').write_bytes((self.root / extra / 'lib.rs').read_bytes())
        shutil.rmtree(self.root / extra)
        external = deepcopy(before['external_tests'][old]); routes = {}
        external['directory'] = destination
        external['origin_directory'] = 'old/extra' if secondary else before['packages'][old]['origin_directory']
        if secondary:
            test_root = 'test/foundation/final-core'
            old_test_root = external['roots'][0]
            (self.root / old_test_root).rename(self.root / test_root)
            external['roots'] = [test_root]
            for row in external['files']:
                original = row['path']; row['path'] = test_root + original[len(old_test_root):]
                routes[original] = dict(destination=row['path'], replacements=[])
        else:
            routes = {row['path']: dict(destination=row['path'], replacements=[]) for row in external['files']}
        self._write_after_mapping(external)
        prior_key = next(iter(before['consolidations']))
        record = dict(primary_directory=extra if secondary else old, destination_directory=destination,
            destination_name=name, reason='Reviewed nested physical consolidation',
            inputs=[dict(package=deepcopy(before['packages'][directory]), external_tests=deepcopy(before['external_tests'].get(directory)))
                    for directory in (old, extra)],
            input_consolidations={old: dict(receipt_id=prior_key, merged_packages=deepcopy(before['merges']))},
            destination_files_sha256=manager.files(path),
            file_routes={old: {relative: dict(destination=relative) for relative in before['packages'][old]['files_sha256']},
                         extra: {'Cargo.toml': dict(archive_base64=base64.b64encode(raw).decode()),
                                 'lib.rs': dict(destination='extra.rs')}},
            new_files={}, external_file_routes=routes, destination_external_tests=external)
        if secondary:
            record['file_routes'][old]['Cargo.toml'] = dict(archive_base64=base64.b64encode(old_manifest).decode())
            record['file_routes'][extra]['Cargo.toml'] = dict(destination='Cargo.toml')
        self._workspace([(destination, name, version)])
        self.refresh(consolidate_plan_data=dict(schema=1, packages=[record])); self.verify()
        self.assertEqual(len(self.catalog['packages']), 1)
        self.assertEqual(len(self.catalog['merges']), 2)
        self.assertEqual(self.catalog['consolidations'][prior_key], before['consolidations'][prior_key])
        self.assertEqual(self.catalog['origin_records_sha256'], before['origin_records_sha256'])
        frozen = (self.root / manager.CATALOG).read_bytes()
        self.refresh(); self.verify()
        self.assertEqual((self.root / manager.CATALOG).read_bytes(), frozen)
        # Loss or re-attribution must never be learned by refresh.
        self.catalog['merges'].pop(); self.save_catalog()
        with self.assertRaisesRegex(ValueError, 'origin coverage'):
            self.refresh()

    def test_nested_primary_refresh_verify_repeat(self):
        self.nested(False)

    def test_nested_secondary_refresh_verify_repeat(self):
        self.nested(True)


if __name__ == '__main__':
    unittest.main()
