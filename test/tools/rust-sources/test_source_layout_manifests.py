"""Ordinary manifest maintenance preserves live contents and immutable layout proof."""
from copy import deepcopy
import importlib
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
from test_source_layout import LayoutFixture, layout
import source_layout_lifecycle_fixture as fixture


class ManifestProjection(LayoutFixture):
    def evolve(self, old):
        target = self.root / layout.route(old, self.scenario.routes)
        raw = target.read_bytes() + b'\n[features]\ndefault = []\nmaintenance = []\n'
        target.write_bytes(raw)
        return raw

    def record(self, old, raw):
        directory = layout.route(old.rsplit('/', 1)[0], self.scenario.routes)
        for owner in [*self.scenario.current['packages'].values(), *self.scenario.current['merges']]:
            if owner['directory'] == directory:
                owner['files_sha256']['Cargo.toml'] = layout.digest(raw)
        self.scenario.write_current()

    def test_grouped_and_stationary_manifests_preserve_edits_and_project_live_hashes(self):
        self.install()
        receipt = (self.root / layout.receipt_path(self.receipt_id)).read_bytes()
        for old in ('src/foundation/alpha/Cargo.toml', 'src/interface/Cargo.toml'):
            with self.subTest(old=old):
                raw = self.evolve(old)
                self.record(old, raw)
                view = self.load()
                logical = view.read(old)
                self.assertIn(b'maintenance = []', logical)
                self.assertNotEqual(logical, self.scenario.before_bytes[old])
                self.assertEqual(layout.rewrite(old, logical, view.moves), raw)
                projected = view.project_catalog(self.scenario.current)
                self.assertEqual(projected['packages'][old.rsplit('/', 1)[0]]['files_sha256']['Cargo.toml'],
                                 layout.digest(logical))
        self.assertEqual((self.root / layout.receipt_path(self.receipt_id)).read_bytes(), receipt)

    def test_stale_catalog_hash_is_not_replaced_with_new_live_manifest_hash(self):
        self.install()
        old = 'src/foundation/alpha/Cargo.toml'
        self.record(old, self.evolve(old))
        previous = self.scenario.current['packages'][self.scenario.routes['src/foundation/alpha']]['files_sha256']['Cargo.toml']
        target = self.root / layout.route(old, self.scenario.routes)
        target.write_bytes(target.read_bytes() + b'another = []\n')
        view = self.load()
        projected = view.project_catalog(self.scenario.current)
        self.assertEqual(projected['packages']['src/foundation/alpha']['files_sha256']['Cargo.toml'], previous)
        self.assertNotEqual(layout.digest(view.read(old)), previous)

    def test_installation_still_requires_exact_root_grouped_and_stationary_manifests(self):
        for old in ('Cargo.toml', 'src/foundation/alpha/Cargo.toml', 'src/interface/Cargo.toml'):
            with self.subTest(old=old):
                self.scenario.reset_live()
                self.evolve(old)
                with self.assertRaises(ValueError):
                    self.install()

    def test_evolved_manifest_cannot_escape_repository(self):
        self.install()
        old = 'src/foundation/alpha/Cargo.toml'
        target = self.root / layout.route(old, self.scenario.routes)
        target.write_bytes(target.read_bytes().replace(b'../../platform/plain', b'../../../../../../outside'))
        with self.assertRaisesRegex(ValueError, 'escapes repository'):
            self.load().read(old)

    def test_manifest_edits_remain_visible_to_original_pinned_proof(self):
        historical = importlib.import_module('historical_test_sources')
        old = 'src/foundation/alpha/Cargo.toml'
        proof = dict(schema_version=1, roots={}, files=[dict(source=old, path=old,
            sha256=layout.digest(self.scenario.before_bytes[old]),
            before_sha256=layout.digest(self.scenario.before_bytes[old]))])
        raw = layout.canonical(proof)
        (self.root / historical.RECEIPT).write_bytes(raw)
        self.receipt['frozen_evidence'][historical.RECEIPT] = layout.digest(raw)
        self.pin()
        corrections = layout.canonical(dict(schema_version=1, files=[]))
        (self.root / historical.CORRECTIONS).write_bytes(corrections)
        routing = importlib.import_module('proof_routing')
        with patch.object(historical, 'RECEIPT_SHA256', layout.digest(raw)), \
                patch.object(historical, 'CORRECTIONS_SHA256', layout.digest(corrections)), \
                patch.object(historical, 'RETIREMENT_ARCHIVES', {}), patch.object(routing, 'RECEIPTS', ()):
            self.install()
            historical.historical_reader(self.root)
            self.record(old, self.evolve(old))
            with self.assertRaisesRegex(ValueError, 'platform current source content drift'):
                historical.historical_reader(self.root)


class ManifestMaintenance(unittest.TestCase):
    def setUp(self):
        self.f = fixture.SourcePolicy('test_complete_fixture_passes_without_registry_cache')
        self.addCleanup(self.f.doCleanups)
        self.f.setUp()
        manifest = self.f.source / 'Cargo.toml'
        manifest.write_bytes(manifest.read_bytes() + b'readme = "../../../docs/mini.txt"\n'
                             b'[features]\ndefault = ["executor", "io"]\nenabled = []\nexecutor = []\nio = []\n')
        (self.f.root / 'docs/mini.txt').write_text('Fixture documentation.\n')
        app = self.f.root / 'src/app/Cargo.toml'
        app.write_bytes(app.read_bytes().replace(b'path = "../foundation/mini"',
            b'path = "../foundation/mini", default-features = false, features = ["enabled", "executor", "io"]'))
        self.f.refresh()
        self.directory = fixture.install(self.f)

    def narrow(self):
        manifest = self.f.source / 'Cargo.toml'
        manifest.write_bytes(manifest.read_bytes().replace(b'default = ["executor", "io"]', b'default = []'))
        app = self.f.root / 'src/app/Cargo.toml'
        app.write_bytes(app.read_bytes().replace(b'["enabled", "executor", "io"]', b'["enabled"]') +
                       b'\n[dev-dependencies.mini]\npath = "../foundation/core-types/mini"\n'
                       b'default-features = false\nfeatures = ["executor"]\n')

    def test_refresh_verify_repeat_preserves_immutable_evidence_and_new_dependency_paths(self):
        before = deepcopy(self.f.catalog)
        marker = before[layout.KEY]
        receipt = (self.f.root / layout.receipt_path(marker)).read_bytes()
        self.narrow()
        with self.assertRaisesRegex(ValueError, 'manifest content drift'):
            self.f.verify()
        self.f.refresh(); self.f.verify()
        catalog = (self.f.root / fixture.manager.CATALOG).read_bytes()
        self.f.refresh(); self.f.verify()
        self.assertEqual((self.f.root / fixture.manager.CATALOG).read_bytes(), catalog)
        self.assertEqual((self.f.root / layout.receipt_path(marker)).read_bytes(), receipt)
        for field in (layout.KEY, *layout.HISTORY_FIELDS):
            self.assertEqual(self.f.catalog.get(field), before.get(field), field)
        view = layout.load(self.f.root)
        self.assertIn(b'default = []', view.read('src/foundation/mini/Cargo.toml'))
        self.assertIn(b'[dev-dependencies.mini]\npath = "../foundation/mini"', view.read('src/app/Cargo.toml'))

    def test_stale_pre_layout_dependency_path_is_rejected(self):
        self.narrow()
        app = self.f.root / 'src/app/Cargo.toml'
        app.write_bytes(app.read_bytes().replace(b'../foundation/core-types/mini', b'../foundation/mini'))
        with self.assertRaisesRegex(ValueError, 'missing local dependency'):
            self.f.refresh()

    def test_package_identity_change_is_not_ordinary_manifest_maintenance(self):
        self.narrow()
        manifest = self.f.source / 'Cargo.toml'
        manifest.write_bytes(manifest.read_bytes().replace(b'version = "1.0.0"', b'version = "2.0.0"'))
        with self.assertRaisesRegex(ValueError, 'identity drift'):
            self.f.refresh()

    def test_original_receipt_tamper_is_not_ordinary_manifest_maintenance(self):
        self.narrow()
        path = self.f.root / layout.receipt_path(self.f.catalog[layout.KEY])
        path.write_bytes(path.read_bytes() + b'tampered')
        with self.assertRaisesRegex(ValueError, 'immutable layout receipt drift'):
            self.f.refresh()


if __name__ == '__main__':
    unittest.main()
