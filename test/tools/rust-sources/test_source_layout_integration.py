"""Historical-validator integration guards using isolated temporary fixtures."""
from copy import deepcopy
import importlib
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE))
from test_source_layout import LayoutFixture, layout
from source_layout_fixture import digest, encoded, material, move

DEFAULT_TOOLS = ((HERE.parents[2] / 'tools/rust-sources') if len(HERE.parents)>=3
                 else Path(os.environ.get('SOURCE_LAYOUT_REPOSITORY', str(Path.cwd()))) / 'tools/rust-sources')
TOOLS=Path(os.environ.get('SOURCE_LAYOUT_TOOLS',str(DEFAULT_TOOLS)))
sys.path.insert(0,str(TOOLS))


class HistoricalIntegration(LayoutFixture):
    def setUp(self):
        super().setUp()
        imported=patch.dict(sys.modules,{'source_layout':layout})
        imported.start();self.addCleanup(imported.stop)
        self.trimming=importlib.import_module('source_trimming')
        self.consolidation=importlib.import_module('source_consolidation')
        self.routing=importlib.import_module('proof_routing')
        self.historical=importlib.import_module('historical_test_sources')
        pins=patch.object(self.routing,'RECEIPTS',())
        pins.start();self.addCleanup(pins.stop)

    def test_trimmed_original_receipt_accepts_exact_relocated_live_owner(self):
        view=self.install()
        with layout.activate(view):
            self.assertEqual(self.trimming.validate(self.root,self.scenario.current),
                             self.scenario.previous['source_trims'])

    def test_trimmed_file_restored_under_new_root_is_not_hidden_by_absent_old_root(self):
        self.install();old='src/foundation/trimmed/src/dead.rs';raw=b'pub fn dead() {}\n'
        material(self.root,{move(old,self.scenario.routes):raw})
        self.scenario.current['packages'][self.scenario.routes['src/foundation/trimmed']]['files_sha256']['src/dead.rs']=digest(raw)
        self.scenario.write_current();view=self.load()
        with layout.activate(view),self.assertRaises(ValueError):
            self.trimming.validate(self.root,self.scenario.current)

    def build_record(self):
        old='src/foundation/trimmed'
        return dict(destination_directory=old,destination_name='trimmed',
                    file_routes={old:{'build.rs':{'kind':'primary-build-script-retirement'}}})

    def test_retired_build_manifest_is_checked_at_current_location(self):
        view=self.install()
        with layout.activate(view):
            self.consolidation._manifest(self.root,self.build_record(),
                                        self.scenario.previous['packages']['src/foundation/trimmed'])

    def test_retired_build_rs_return_at_grouped_root_rejected(self):
        self.install();old='src/foundation/trimmed/build.rs';raw=b'fn main() {}\n'
        material(self.root,{move(old,self.scenario.routes):raw})
        self.scenario.current['packages'][self.scenario.routes['src/foundation/trimmed']]['files_sha256']['build.rs']=digest(raw)
        self.scenario.write_current();view=self.load()
        with layout.activate(view),self.assertRaises(ValueError):
            self.consolidation._manifest(self.root,self.build_record(),
                                        self.scenario.previous['packages']['src/foundation/trimmed'])

    def prepare_direct_platform_proof(self):
        self.install()
        corrections=encoded({'schema_version':1,'files':[]})
        material(self.root,{self.historical.CORRECTIONS:corrections})
        proof=(self.root/self.historical.RECEIPT).read_bytes()
        for name,value in [('RECEIPT_SHA256',digest(proof)),('CORRECTIONS_SHA256',digest(corrections)),('RETIREMENT_ARCHIVES',{})]:
            p=patch.object(self.historical,name,value);p.start();self.addCleanup(p.stop)
        return self.load()

    def test_direct_platform_owner_without_old_routing_receipt_still_reads_live(self):
        view=self.prepare_direct_platform_proof();old='src/foundation/plain/src/lib.rs'
        with layout.activate(view):
            read=self.historical.historical_reader(self.root)
            self.assertEqual(read(old),self.scenario.before_bytes[old])

    def test_direct_platform_live_tamper_rejected_even_if_catalog_refreshed(self):
        self.prepare_direct_platform_proof();old='src/foundation/plain/src/lib.rs'
        raw=b'pub fn identity(_: u8) -> u8 { 99 }\n'
        material(self.root,{move(old,self.scenario.routes):raw})
        self.scenario.current['packages'][self.scenario.routes['src/foundation/plain']]['files_sha256']['src/lib.rs']=digest(raw)
        self.scenario.write_current();view=self.load()
        with layout.activate(view),self.assertRaises(ValueError):
            self.historical.historical_reader(self.root)

    def test_direct_platform_missing_live_file_not_recovered_from_frozen_proof(self):
        view=self.prepare_direct_platform_proof();old='src/foundation/plain/src/lib.rs'
        (self.root/move(old,self.scenario.routes)).unlink()
        with layout.activate(view),self.assertRaises((ValueError,FileNotFoundError)):
            self.historical.historical_reader(self.root)

    def test_existing_routed_owner_current_package_returns_grouped_directory(self):
        view=self.install()
        with layout.activate(view):
            routed=self.routing.View(self.root)
            routed.packages['src/foundation/legacy-alpha']=deepcopy(
                self.scenario.previous['packages']['src/foundation/alpha'])
            package=routed.current_package('src/foundation/legacy-alpha')
            self.assertEqual(package['directory'],self.scenario.routes['src/foundation/alpha'])
            self.assertTrue((self.root/package['directory']/'Cargo.toml').is_file())
            self.assertEqual(routed.packages['src/foundation/legacy-alpha']['directory'],
                             'src/foundation/alpha','current lookup must not mutate historical view')

    def test_routing_inventory_uses_logical_paths_and_new_physical_files(self):
        view=self.install();old='src/foundation/plain'
        with layout.activate(view):
            proof=self.routing.load(self.root,self.scenario.current)
            self.assertEqual(proof.inventory(old),self.scenario.full_inventory(old))
            self.assertEqual(proof.current_path(old+'/src/lib.rs'),move(old+'/src/lib.rs',self.scenario.routes))


if __name__=='__main__':unittest.main(verbosity=2)
