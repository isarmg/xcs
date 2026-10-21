"""Only authenticated secondary root build scripts can become inert receipts."""
import base64
from copy import deepcopy
import unittest
from unittest.mock import patch
import test_source_consolidation as fixtures
from test_source_consolidation import consolidation, digest, inventory


def encoded(raw):
    return base64.b64encode(raw).decode('ascii')


class ArchivedBuildScripts(unittest.TestCase):
    apply = fixtures.SourceConsolidation.apply
    catalog = fixtures.SourceConsolidation.catalog
    reject = fixtures.SourceConsolidation.reject

    def setUp(self):
        fixtures.SourceConsolidation.setUp(self)
        self.directory = self.old[1]
        self.raw = b'fn main() {\n    #[cfg(feature = "pyo3")]\n    pyo3_build_config::use_pyo3_cfgs();\n}\n'
        self.reason = 'Retire the optional Python bridge whose provider is absent; preserve its exact build entrypoint as inert historical evidence.'
        self.manifest(self.manifests[self.directory] + b'build = "build.rs"\n')
        self.source(self.raw)
        self.destination_manifest(b'[package]\nname="crypto-core"\nversion="1.0.0"\nbuild=false\n')

    @property
    def route(self):
        return self.item['file_routes'][self.directory]['build.rs']

    def manifest(self, raw):
        self.manifests[self.directory] = raw
        for package in (self.previous['packages'][self.directory], self.item['inputs'][1]['package']):
            package['files_sha256']['Cargo.toml'] = digest(raw)
        self.item['file_routes'][self.directory]['Cargo.toml'] = {'archive_base64': encoded(raw)}

    def source(self, raw):
        for package in (self.previous['packages'][self.directory], self.item['inputs'][1]['package']):
            package['files_sha256']['build.rs'] = digest(raw)
        self.item['file_routes'][self.directory]['build.rs'] = dict(kind='archived-build-script',
            sha256=digest(raw), archive_base64=encoded(raw), reason=self.reason,
            destination_manifest_base64=self.item['file_routes'][self.directory].get('build.rs', {}).get('destination_manifest_base64', ''))

    def destination_manifest(self, raw):
        (self.root / self.destination / 'Cargo.toml').write_bytes(raw)
        self.item['destination_files_sha256'] = inventory(self.root / self.destination)
        self.route['destination_manifest_base64'] = encoded(raw)

    def test_exact_secondary_script_is_immutable_inert_history(self):
        original = deepcopy(self.plan)
        result = self.apply()
        self.assertEqual(self.plan, original)
        self.assertNotIn('build.rs', result[0][self.destination]['files_sha256'])
        self.assertEqual(base64.b64decode(self.route['archive_base64']), self.raw)
        catalog = self.catalog(result)
        consolidation.validate_history(self.root, catalog)
        next(iter(catalog['consolidations'].values()))['file_routes'][self.directory]['build.rs']['reason'] += ' changed'
        with self.assertRaisesRegex(ValueError, 'hash drift'):
            consolidation.validate_history(self.root, catalog)

    def test_implicit_default_build_declaration_supported(self):
        self.manifest(self.manifests[self.directory].replace(b'build = "build.rs"\n', b''))
        self.apply()

    def test_primary_script_rejected(self):
        self.item['primary_directory'] = self.directory
        self.reject('secondary|redundant input')

    def test_only_exact_root_build_path_supported(self):
        for path in ('src/build.rs', 'src/lib.rs', 'LICENSE', 'legal/build.rs', 'test/build.rs', './build.rs', 'custom.rs'):
            with self.subTest(path=path), self.assertRaisesRegex(ValueError, 'secondary root'):
                consolidation._build_script_archive(path, self.route, digest(self.raw), self.directory,
                    self.item['inputs'][1]['package'], self.item)

    def test_original_manifest_identity_and_build_ownership_required(self):
        original = self.manifests[self.directory]
        for raw in (original.replace(b'block-buffer', b'wrong'), original.replace(b'2.0.0', b'9.0.0'),
                    original.replace(b'"build.rs"', b'false'), original.replace(b'"build.rs"', b'"custom.rs"'),
                    original + b'links="native"\n', original + b'license-file="build.rs"\n',
                    original + b'license-file="src/../build.rs"\n'):
            with self.subTest(raw=raw):
                self.manifest(raw)
                self.reject('identity/build/legal')
        self.manifest(original)

    def test_recorded_legal_material_rejected(self):
        self.item['inputs'][1]['package']['license_materials'] = [self.directory + '/build.rs']
        self.reject('legal')

    def test_manifest_must_be_exact_archive_not_route(self):
        self.item['file_routes'][self.directory]['Cargo.toml'] = {'destination': 'Cargo.toml'}
        self.reject('archived input manifest')

    def test_tampered_original_or_destination_manifest_rejected(self):
        self.item['file_routes'][self.directory]['Cargo.toml']['archive_base64'] = encoded(b'[package]\nname="x"\n')
        self.reject('before hash')
        self.manifest(self.manifests[self.directory])
        self.route['destination_manifest_base64'] = encoded(b'[package]\nname="x"\n')
        self.reject('before hash')

    def test_destination_build_and_links_forbidden(self):
        original = (self.root / self.destination / 'Cargo.toml').read_bytes()
        for raw in (original.replace(b'build=false\n', b''), original.replace(b'false', b'true'),
                    original.replace(b'false', b'"custom.rs"'), original + b'links="native"\n'):
            self.destination_manifest(raw)
            self.reject('inert')

    def test_destination_script_or_exact_byte_copy_forbidden(self):
        for path in ('build.rs', 'src/retained.rs'):
            self.item['destination_files_sha256'][path] = digest(self.raw)
            self.item['new_files'][path] = digest(self.raw)
            self.reject('inert')
            del self.item['destination_files_sha256'][path], self.item['new_files'][path]

    def test_current_destination_cannot_enable_build_later(self):
        catalog = self.catalog()
        (self.root / self.destination / 'Cargo.toml').write_text('[package]\nname="crypto-core"\nversion="1.0.0"\nbuild="custom.rs"\n')
        with self.assertRaisesRegex(ValueError, 'inert'):
            consolidation.validate_history(self.root, catalog)

    def test_source_hash_bytes_utf8_and_reason_guards(self):
        for raw in (b'', b'\xff'):
            self.source(raw)
            self.reject()
        self.source(self.raw)
        for field, value in (('sha256', '0' * 64), ('archive_base64', '!!!'), ('reason', 'short')):
            original = deepcopy(self.route)
            self.route[field] = value
            self.reject()
            self.route.clear(); self.route.update(original)

    def test_source_and_aggregate_bounds_shared(self):
        for field, value in (('MAX_ARCHIVE_BYTES', len(self.raw) - 1),
                             ('MAX_SOURCE_ARCHIVES', 0), ('MAX_SOURCE_ARCHIVE_TOTAL', len(self.raw) - 1)):
            with patch.object(consolidation, field, value):
                self.reject('bound')

    def test_old_source_route_cannot_archive_build_entrypoint(self):
        self.route['kind'] = 'archived-source'
        del self.route['destination_manifest_base64']
        self.reject('non-entrypoint')

    def test_unknown_or_missing_fields_rejected(self):
        original = deepcopy(self.route)
        for field in original:
            self.route.clear(); self.route.update(original); del self.route[field]
            self.reject()
        self.route.clear(); self.route.update(original); self.route['destination'] = 'src/buffer.rs'
        self.reject()


if __name__ == '__main__':
    unittest.main()
