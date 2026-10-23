"""Post-layout upgrades preserve frozen evidence and require explicit proofs."""
from copy import deepcopy
import json
from pathlib import Path
import sys
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parent))
import source_layout_lifecycle_fixture as fixture
from source_layout_lifecycle_fixture import manager, layout, install, sha


class GroupedUpgrades(unittest.TestCase):
    def setUp(self):
        self.f = fixture.SourcePolicy('test_complete_fixture_passes_without_registry_cache')
        self.addCleanup(self.f.doCleanups)
        self.f.setUp()

    def prepare(self, external=False):
        f = self.f
        if external:
            f.install_external_tests(); f.refresh(); f.verify()
        self.directory = install(f)
        raw = (f.root / manager.ORIGINS).read_bytes()
        (f.root / ('compliance/source-layout/origins-' + sha(raw) + '.json')).write_bytes(raw)
        # Existing fixture upgrade helper addresses the old key only.
        before = deepcopy(f.catalog['packages'][self.directory])
        f.catalog['packages']['src/foundation/mini'] = before
        try:
            plan = f.upgrade_mini()
        finally:
            del f.catalog['packages']['src/foundation/mini']
        plan['packages'][0].update(directory=self.directory, package=before)
        if external:
            plan['packages'][0].update(external_action='remove', external_tests=deepcopy(f.catalog['external_tests'][self.directory]))
            f.remove_external_tests()
        return plan

    def test_explicit_upgrade_repeat_and_verify(self):
        plan = self.prepare(); f = self.f
        f.refresh(upgrade_plan_data=plan); f.verify(); f.refresh(); f.verify()
        self.assertEqual(f.catalog['packages'][self.directory]['version'], '1.1.0')

    def test_archived_external_identity_and_successor_separation(self):
        plan = self.prepare(True); f = self.f
        f.refresh(upgrade_plan_data=plan); f.verify()
        old = deepcopy(f.catalog['superseded'])
        source = f.source / 'src/lib.rs'; data = source.read_bytes()
        f.external_root.mkdir()
        target = f.external_root / 'successor.rs'; target.write_bytes(data)
        owner = dict(directory=self.directory, roots=['test/foundation/mini'], files=[dict(source=self.directory+'/src/lib.rs', path='test/foundation/mini/successor.rs', source_sha256_before=sha(data), sha256=sha(data))])
        f.external_mapping.write_text(json.dumps(dict(schema_version=1, owners=[owner])))
        source.unlink(); f.refresh(); f.verify()
        self.assertEqual(old, f.catalog['superseded'])

    def test_unplanned_append_rejected(self):
        self.prepare()
        with self.assertRaises(ValueError): self.f.refresh()

    def test_old_origin_mutation_rejected(self):
        plan = self.prepare(); f = self.f
        f.origin_records[0]['upstream']['version'] = '9.9.9'
        f.write_json(manager.ORIGINS, dict(packages=f.origin_records))
        with self.assertRaises(ValueError): f.refresh(upgrade_plan_data=plan)

    def test_unrelated_append_rejected(self):
        plan = self.prepare(); f = self.f
        extra = deepcopy(f.origin_records[-1]); extra['origin_directory'] = 'other/unapproved'
        f.origin_records.append(extra); f.write_json(manager.ORIGINS, dict(packages=f.origin_records))
        with self.assertRaises(ValueError): f.refresh(upgrade_plan_data=plan)

    def test_changed_upstream_source_rejected(self):
        plan = self.prepare(); (self.f.source/'src/lib.rs').write_text('pub fn forged() {}\n')
        with self.assertRaises(ValueError): self.f.refresh(upgrade_plan_data=plan)

    def test_fake_prior_package_rejected(self):
        plan = self.prepare(); plan['packages'][0]['package']['version'] = 'forged'
        with self.assertRaises(ValueError): self.f.refresh(upgrade_plan_data=plan)

    def test_snapshot_mutation_rejected(self):
        plan = self.prepare(); next((self.f.root/'compliance/source-layout').glob('origins-*.json')).write_text('{}')
        with self.assertRaises(ValueError): self.f.refresh(upgrade_plan_data=plan)

    def test_rehashed_chain_loses_original_external_labels_rejected(self):
        plan = self.prepare(True); f = self.f
        f.refresh(upgrade_plan_data=plan)
        receipt = f.catalog['superseded']['old/mini']; receipt['external_tests']['files'][0]['source'] += '.forged'
        f.catalog['superseded_records_sha256'] = manager.origin_hashes(f.catalog['superseded']); f.save_catalog()
        with self.assertRaises(ValueError): f.verify()

    def test_rehashed_first_predecessor_inventory_mutation_rejected(self):
        plan = self.prepare(); f = self.f
        f.refresh(upgrade_plan_data=plan)
        receipt = f.catalog['superseded']['old/mini']; receipt['package']['files_sha256']['lib.rs'] = '0' * 64
        f.catalog['superseded_records_sha256'] = manager.origin_hashes(f.catalog['superseded']); f.save_catalog()
        with self.assertRaises(ValueError): f.verify()

    def test_extra_source_in_authenticated_prefix_rejected(self):
        plan = self.prepare(); (self.f.source/'src/extra.rs').write_text('pub fn extra() {}\n')
        with self.assertRaises(ValueError): self.f.refresh(upgrade_plan_data=plan)

    def test_missing_source_in_authenticated_prefix_rejected(self):
        plan = self.prepare(); (self.f.source/'src/helper.rs').unlink()
        with self.assertRaises(ValueError): self.f.refresh(upgrade_plan_data=plan)

    def test_unrelated_history_mutation_rejected(self):
        plan = self.prepare(); f = self.f
        f.refresh(upgrade_plan_data=plan)
        f.catalog['renames']['forged'] = {}; f.write_json(manager.CATALOG, f.catalog)
        with self.assertRaises(ValueError): f.verify()

    def test_upgraded_logical_read_requires_current_inventory_hash(self):
        plan = self.prepare(); f = self.f
        f.refresh(upgrade_plan_data=plan)
        (f.source/'src/helper.rs').write_text('pub fn forged() {}\n')
        with self.assertRaises(ValueError): layout.load(f.root).read('src/foundation/mini/src/helper.rs')

    def test_upgraded_unknown_logical_read_rejected(self):
        plan = self.prepare(); f = self.f
        f.refresh(upgrade_plan_data=plan)
        (f.source/'src/unknown.rs').write_text('pub fn unknown() {}\n')
        with self.assertRaises(ValueError): layout.load(f.root).read('src/foundation/mini/src/unknown.rs')

if __name__ == '__main__': unittest.main()
