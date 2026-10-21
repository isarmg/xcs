"""Consolidation after prior moves/renames needs exact immutable lineage."""
from copy import deepcopy
import sys
from pathlib import Path
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / 'tools/rust-sources'))
import test_source_consolidation as fixtures
from test_source_consolidation import digest
import source_consolidation as consolidation
import source_history_chain as chain
import source_naming
import source_moves


class SourceHistoryChain(unittest.TestCase):
    def setUp(self):
        fixtures.SourceConsolidation.setUp(self)

    apply = fixtures.SourceConsolidation.apply
    catalog = fixtures.SourceConsolidation.catalog
    reject = fixtures.SourceConsolidation.reject

    def bind(self):
        self.item['input_history'] = chain.expected(self.previous,
            {i['package']['directory']: i['package'] for i in self.item['inputs']}, self.destination)

    def move(self):
        directory = self.old[1]
        before = deepcopy(self.previous['packages'][directory])
        # This old file was legitimately trimmed before the consolidation.
        before['files_sha256']['src/obsolete.rs'] = digest(b'old helper')
        after = deepcopy(before['files_sha256'])
        after['src/renamed.rs'] = after.pop('src/obsolete.rs')
        row = dict(directory=directory, package=before, merged_packages=[], files_sha256=after,
            file_moves={'src/obsolete.rs': 'src/renamed.rs'}, reason='Reviewed internal helper name',
            external_tests=None, destination_external_tests=None, external_file_moves={})
        self.previous['source_moves'] = {source_moves.receipt_id(row): row}
        self.bind()
        return directory, row

    def rename(self):
        directory = self.old[1]
        before = deepcopy(self.previous['packages'][directory])
        before['directory'] = 'src/foundation/original-buffer'
        before['name'] = 'original-buffer'
        package = self.previous['packages'][directory]
        # Fixture destination name must be valid kebab-case and match basename.
        new = 'src/foundation/current-buffer'
        self.previous['packages'][new] = self.previous['packages'].pop(directory)
        package.update(directory=new, name='current-buffer')
        self.item['inputs'][1]['package'] = deepcopy(package)
        self.item['file_routes'][new] = self.item['file_routes'].pop(directory)
        row = dict(package=before, destination_directory=new, destination_name='current-buffer',
            reason='Reviewed functional package identity', files_sha256=deepcopy(package['files_sha256']),
            external_tests=None, destination_external_tests=None)
        self.previous['renames'] = {before['directory']: row}
        self.previous['rename_records_sha256'] = source_naming.hashes(self.previous['renames'])
        self.bind()
        return new, row

    def test_move_history_allows_later_trimmed_files(self):
        directory, row = self.move()
        catalog = self.catalog()
        self.assertEqual(catalog['source_moves'], self.previous['source_moves'])
        self.assertNotIn('src/renamed.rs', self.item['inputs'][1]['package']['files_sha256'])
        consolidation.validate_history(self.root, catalog)

    def test_rename_history_resolves_merged_successor(self):
        directory, row = self.rename()
        catalog = self.catalog()
        self.assertFalse((self.root / directory).exists())
        self.assertEqual(source_naming.validate_history(self.root, catalog), self.previous['renames'])
        self.assertEqual(catalog['merges'][0]['origin_directory'], row['package']['origin_directory'])

    def two_renames(self):
        first, _ = self.rename()
        old, new = self.old[2], 'src/foundation/current-common'
        package = self.previous['packages'].pop(old)
        before = deepcopy(package)
        before.update(directory='src/foundation/original-common', name='original-common')
        package.update(directory=new, name='current-common')
        self.previous['packages'][new] = package
        self.item['inputs'][2]['package'] = deepcopy(package)
        self.item['file_routes'][new] = self.item['file_routes'].pop(old)
        row = dict(package=before, destination_directory=new, destination_name='current-common',
            reason='Reviewed common identity', files_sha256=deepcopy(package['files_sha256']),
            external_tests=None, destination_external_tests=None)
        self.previous['renames'][before['directory']] = row
        self.previous['rename_records_sha256'] = source_naming.hashes(self.previous['renames'])
        self.bind()
        return first, new

    def test_consumed_renames_share_one_fresh_validation_pass(self):
        from unittest.mock import patch
        self.two_renames(); catalog = self.catalog()
        with patch.object(consolidation, 'validate_history', wraps=consolidation.validate_history) as check:
            for expected_calls in (1, 2):
                self.assertEqual(source_naming.validate_history(self.root, catalog), catalog['renames'])
                self.assertEqual(check.call_count, expected_calls)

    def test_second_call_rechecks_changed_manifest_at_same_path(self):
        self.two_renames(); catalog = self.catalog()
        source_naming.validate_history(self.root, catalog)
        manifest = self.root / self.destination / 'Cargo.toml'
        manifest.write_text('[package]\nname="tampered"\nversion="1.0.0"\n')
        with self.assertRaisesRegex(ValueError, 'manifest name/version'):
            source_naming.validate_history(self.root, catalog)

    def test_second_call_rechecks_returned_path(self):
        self.two_renames(); catalog = self.catalog()
        source_naming.validate_history(self.root, catalog)
        (self.root / 'src/foundation/current-common').mkdir()
        with self.assertRaisesRegex(ValueError, 'consolidated source root'):
            source_naming.validate_history(self.root, catalog)

    def test_second_call_rechecks_mutated_receipt(self):
        self.two_renames(); catalog = self.catalog()
        source_naming.validate_history(self.root, catalog)
        next(iter(catalog['consolidations'].values()))['reason'] += ' tampered'
        with self.assertRaisesRegex(ValueError, 'receipt hash drift'):
            source_naming.validate_history(self.root, catalog)

    def test_batch_does_not_ignore_unqueried_malformed_consolidation(self):
        self.two_renames(); catalog = self.catalog()
        bad = {'malformed': 'unrelated'}
        catalog['consolidations'][consolidation.receipt_id(bad)] = bad
        catalog['consolidation_records_sha256'] = consolidation.hashes(catalog['consolidations'])
        with self.assertRaisesRegex(ValueError, 'consolidation.*fields'):
            source_naming.validate_history(self.root, catalog)

    def test_missing_move_binding_rejected(self):
        self.move(); del self.item['input_history']
        self.reject('exact immutable')

    def test_missing_rename_binding_rejected(self):
        self.rename(); del self.item['input_history']
        self.reject('exact immutable')

    def test_tampered_move_id_rejected(self):
        self.move(); key, row = next(iter(self.previous['source_moves'].items()))
        row['reason'] += ' changed'; self.bind()
        self.reject('receipt drift')

    def test_tampered_rename_hash_rejected(self):
        self.rename(); next(iter(self.previous['renames'].values()))['reason'] += ' changed'
        self.bind(); self.reject('receipt drift')

    def test_changed_embedded_receipt_rejected(self):
        directory, row = self.rename()
        next(iter(self.item['input_history'][directory]['renames'].values()))['reason'] += ' changed'
        self.reject('exact immutable')

    def test_missing_origin_rejected(self):
        directory, row = self.move(); row['package'].pop('origin_directory')
        self.previous['source_moves'] = {source_moves.receipt_id(row): row}; self.bind()
        self.reject('identity')

    def test_wrong_move_origin_rejected(self):
        directory, row = self.move(); row['package']['origin_directory'] = 'vendor/wrong'
        self.previous['source_moves'] = {source_moves.receipt_id(row): row}; self.bind()
        self.reject('lineage')

    def test_wrong_rename_origin_rejected(self):
        directory, row = self.rename(); row['package']['origin_directory'] = 'vendor/wrong'
        self.previous['rename_records_sha256'] = source_naming.hashes(self.previous['renames']); self.bind()
        self.reject('same-origin')

    def test_stale_current_snapshot_rejected(self):
        self.move(); self.item['inputs'][1]['package']['files_sha256']['src/lib.rs'] = 'a' * 64
        self.reject('exact previous')

    def test_route_cannot_resurrect_obsolete_historical_file(self):
        directory, row = self.move()
        self.item['file_routes'][directory]['src/renamed.rs'] = {'destination': 'src/buffer.rs'}
        self.reject('every before')

    def test_wrong_binding_directory_rejected(self):
        directory, row = self.move()
        self.item['input_history']['src/foundation/unknown'] = self.item['input_history'].pop(directory)
        self.reject('history roots')

    def test_move_path_escape_rejected(self):
        directory, row = self.move(); row['file_moves']['src/obsolete.rs'] = '../escape.rs'
        self.previous['source_moves'] = {source_moves.receipt_id(row): row}; self.bind()
        self.reject('file_moves|inventory')

    def test_ambiguous_renames_rejected(self):
        directory, row = self.rename(); other = deepcopy(row)
        other['package']['directory'] = 'src/foundation/another-buffer'
        self.previous['renames'][other['package']['directory']] = other
        self.previous['rename_records_sha256'] = source_naming.hashes(self.previous['renames']); self.bind()
        self.reject('ambiguous')

    def test_old_rename_root_returned_rejected(self):
        directory, row = self.rename(); (self.root / row['package']['directory']).mkdir()
        self.reject('returned')

    def test_old_rename_test_root_returned_rejected(self):
        directory, row = self.rename()
        (self.root / 'test/foundation/original-buffer').mkdir(parents=True)
        self.reject('external test root')

    def test_current_input_root_returned_rejected(self):
        directory, row = self.rename(); (self.root / directory).mkdir()
        self.reject('consolidated source root')

    def test_persistent_missing_history_rejected(self):
        self.move(); catalog = self.catalog(); catalog['source_moves'] = {}
        with self.assertRaisesRegex(ValueError, 'exact immutable'):
            consolidation.validate_history(self.root, catalog)

    def test_persistent_missing_consolidation_rejected(self):
        self.rename(); catalog = self.catalog()
        catalog.pop('consolidations'); catalog.pop('consolidation_records_sha256')
        with self.assertRaisesRegex(ValueError, 'same-origin'):
            source_naming.validate_history(self.root, catalog)

    def test_persistent_missing_merged_origin_rejected(self):
        self.rename(); catalog = self.catalog(); catalog['merges'].pop(0)
        with self.assertRaisesRegex(ValueError, 'origin coverage'):
            source_naming.validate_history(self.root, catalog)

    def test_persistent_changed_binding_hash_rejected(self):
        self.rename(); catalog = self.catalog()
        next(iter(catalog['consolidations'].values()))['input_history'] = {}
        with self.assertRaisesRegex(ValueError, 'receipt hash drift'):
            source_naming.validate_history(self.root, catalog)

    def test_link_bound_rejected(self):
        from unittest.mock import patch
        self.move()
        with patch.object(chain, 'MAX_LINKS', 0):
            self.reject('bound')

    def test_empty_link_rejected(self):
        directory, row = self.move()
        self.item['input_history'][directory] = {'renames': {}, 'source_moves': {}}
        self.reject('explicit')

    def test_unknown_link_fields_rejected(self):
        directory, row = self.move()
        self.item['input_history'][directory]['waive'] = True
        self.reject('explicit')

    def test_history_destination_collision_rejected(self):
        directory, row = self.move()
        row['directory'] = self.destination
        row['package']['directory'] = self.destination
        row['package']['origin_directory'] = 'vendor/other'
        self.previous['source_moves'] = {source_moves.receipt_id(row): row}
        with self.assertRaisesRegex(ValueError, 'destination intersects'):
            self.bind()

    def test_renamed_primary_resolves_after_moving_again_by_consolidation(self):
        directory, row = self.rename()
        self.item['primary_directory'] = directory
        self.item['file_routes'][self.old[0]]['Cargo.toml'] = {
            'archive_base64': __import__('base64').b64encode(self.manifests[self.old[0]]).decode()}
        self.item['file_routes'][directory]['Cargo.toml'] = {'destination': 'Cargo.toml'}
        manifest = self.root / self.destination / 'Cargo.toml'
        manifest.write_text(manifest.read_text().replace('1.0.0', '2.0.0'))
        self.item['destination_files_sha256']['Cargo.toml'] = digest(manifest.read_bytes())
        catalog = self.catalog()
        source_naming.validate_history(self.root, catalog)
        self.assertEqual(catalog['packages'][self.destination]['origin_directory'], row['package']['origin_directory'])

    def test_renamed_successor_checks_actual_consolidated_manifest(self):
        self.rename(); catalog = self.catalog()
        manifest = self.root / self.destination / 'Cargo.toml'
        manifest.write_text(manifest.read_text().replace('crypto-core', 'wrong'))
        with self.assertRaisesRegex(ValueError, 'manifest'):
            source_naming.validate_history(self.root, catalog)

    def test_move_before_rename_is_not_silently_reinterpreted(self):
        directory, rename = self.rename()
        old = deepcopy(rename['package'])
        files = deepcopy(old['files_sha256'])
        files['src/renamed.rs'] = files.pop('src/lib.rs')
        move = dict(directory=old['directory'], package=old, merged_packages=[],
                    files_sha256=files, file_moves={'src/lib.rs': 'src/renamed.rs'},
                    external_file_moves={}, external_tests=None, destination_external_tests=None,
                    reason='Earlier internal move under original package identity')
        self.previous['source_moves'] = {source_moves.receipt_id(move): move}
        self.bind(); self.reject('lineage')


class RenamedExternalTransfer(unittest.TestCase):
    def setUp(self):
        import test_external_transfers as external_fixture
        external_fixture.ExactOriginTransfer.setUp(self)
        item = self.record['inputs'][1]
        before_package = dict(item['package'], name='second', version='1.0.0',
                              upstream={'name': 'second', 'version': '1.0.0'})
        before_owner = deepcopy(item['external_tests'])
        current = deepcopy(before_package)
        current.update(directory='src/foundation/renamed-second', name='renamed-second')
        owner = deepcopy(before_owner)
        owner.update(directory=current['directory'], roots=['test/foundation/renamed-second'])
        old_path = owner['files'][0]['path']
        new_path = 'test/foundation/renamed-second/unit.rs'
        owner['files'][0]['path'] = new_path
        item.update(package=current, external_tests=owner)
        self.record['external_file_routes'][new_path] = self.record['external_file_routes'].pop(old_path)
        row = dict(package=before_package, destination_directory=current['directory'],
                   destination_name=current['name'], reason='Reviewed rename',
                   files_sha256=current['files_sha256'], external_tests=before_owner,
                   destination_external_tests=owner)
        self.record['input_history'] = {current['directory']: {
            'renames': {before_package['directory']: row}, 'source_moves': {}}}

    def test_exact_old_source_label_survives_rename_and_transfer(self):
        import external_transfers
        external_transfers.validate(self.root, self.record)
        self.assertEqual(self.owner['files'][1]['source'], 'src/foundation/second/src/lib.rs')

    def test_unrelated_old_prefix_rejected(self):
        import external_transfers
        self.record['inputs'][1]['external_tests']['files'][0]['source'] = 'src/foundation/other/src/lib.rs'
        with self.assertRaisesRegex(ValueError, 'source'):
            external_transfers.validate(self.root, self.record)

    def test_missing_prefix_binding_rejected(self):
        import external_transfers
        self.record.pop('input_history')
        with self.assertRaisesRegex(ValueError, 'source'):
            external_transfers.validate(self.root, self.record)


if __name__ == '__main__': unittest.main()
