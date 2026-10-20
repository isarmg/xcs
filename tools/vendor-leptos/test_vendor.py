"""Offline fixture regressions for source provenance and local resolution."""
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('vendor', Path(__file__).with_name('vendor.py'))
vendor = importlib.util.module_from_spec(spec)
spec.loader.exec_module(vendor)


class VendorTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.dest = self.root / 'vendor/web/dependencies'
        self.dest.mkdir(parents=True)
        self.globals = patch.multiple(vendor, ROOT=self.root, DEST=self.dest, PROVENANCE=self.dest / 'provenance.json')
        self.globals.start()
        self.addCleanup(self.globals.stop)
        self.directory = self.dest / 'fixture-1.0.0'
        self.directory.mkdir()
        manifest = '[package]\nname="fixture"\nversion="1.0.0"\n'
        (self.directory / 'Cargo.toml').write_text(manifest)
        (self.directory / 'lib.rs').write_text('// upstream\n')
        self.packages = {'fixture': {'version': '1.0.0', 'upstream_files': {'lib.rs': vendor.sha(b'// upstream\n')}, 'maintained_manifest_sha256': vendor.sha(manifest.encode())}}
        (self.dest / 'provenance.json').write_text(json.dumps({'packages': self.packages}))
        (self.dest / 'patches.json').write_text('{"packages": {}}')
        for consumer in ['web', 'demos/branch-system']:
            folder = self.root / consumer
            folder.mkdir(parents=True, exist_ok=True)
            (folder / 'Cargo.lock').write_text('[[package]]\nname="fixture"\nversion="1.0.0"\n')

    def verify(self):
        with contextlib.redirect_stdout(io.StringIO()):
            vendor.verify()

    def test_clean_fixture(self):
        self.verify()

    def test_extra_file_rejected(self):
        (self.directory / 'extra.rs').write_text('// extra')
        with self.assertRaisesRegex(SystemExit, 'Unrecorded file'):
            self.verify()

    def test_symlink_rejected(self):
        (self.directory / 'link').symlink_to('lib.rs')
        with self.assertRaisesRegex(SystemExit, 'Symlink not allowed'):
            self.verify()

    def test_source_mutation_rejected(self):
        (self.directory / 'lib.rs').write_text('// changed')
        with self.assertRaisesRegex(SystemExit, 'Unrecorded change'):
            self.verify()

    def test_patch_requires_reason_and_retains_upstream_hash(self):
        new = '// maintained'
        (self.directory / 'lib.rs').write_text(new)
        record = {'lib.rs': {'sha256': vendor.sha(new.encode()), 'reason': 'Regression fix for project behavior'}}
        (self.dest / 'patches.json').write_text(json.dumps({'packages': {'fixture': record}}))
        self.verify()
        self.assertEqual(self.packages['fixture']['upstream_files']['lib.rs'], vendor.sha(b'// upstream\n'))
        record['lib.rs']['reason'] = ''
        with self.assertRaises(ValueError):
            vendor.patched_files({}, record)

    def test_duplicate_registry_copy_rejected(self):
        lock = self.root / 'web/Cargo.lock'
        with lock.open('a') as f:
            f.write('[[package]]\nname="fixture"\nversion="1.0.0"\nsource="registry+example"\n')
        with self.assertRaisesRegex(SystemExit, 'expected exactly one'):
            self.verify()

    def test_manifest_patch_verified_without_rewriting_import_provenance(self):
        manifest = self.directory / 'Cargo.toml'
        original = manifest.read_bytes()
        updated = original + b'\n[dependencies.paste]\npackage="pastey"\nversion="=0.2.3"\n'
        manifest.write_bytes(updated)
        with self.assertRaisesRegex(SystemExit, 'Unrecorded change'):
            self.verify()
        record = {'Cargo.toml': {'sha256': vendor.sha(updated), 'reason': 'Replace unmaintained paste with compatible pastey'}}
        (self.dest / 'patches.json').write_text(json.dumps({'packages': {'fixture': record}}))
        self.verify()
        provenance = json.loads((self.dest / 'provenance.json').read_text())
        self.assertEqual(provenance['packages']['fixture']['maintained_manifest_sha256'], vendor.sha(original))
        manifest.write_bytes(updated + b'# unrecorded edit\n')
        with self.assertRaisesRegex(SystemExit, 'Unrecorded change'):
            self.verify()

    def test_metadata_path_rejected(self):
        entries = [{'name': 'fixture', 'version': '1.0.0', 'source': None, 'manifest_path': '/elsewhere/Cargo.toml'}]
        self.assertIn('unexpected manifest path', '\n'.join(vendor.check_resolution(entries, self.packages, 'fixture', metadata=True)))

    def test_wires_normal_dev_and_target_edges(self):
        source = '\n'.join(['[dependencies.fixture]', 'version="1"', '[dev-dependencies.fixture]', 'version="1"', '[target.\'cfg(unix)\'.dependencies.fixture]', 'version="1"'])
        self.assertEqual(vendor.wire(source, self.packages).count('path = "../fixture-1.0.0"'), 3)

    def test_import_from_seed_with_path_only_lock(self):
        names = ['any_spawner', 'const_str_slice_concat', 'either_of', 'leptos_dom', 'leptos_hot_reload', 'next_tuple', 'oco_ref', 'or_poisoned', 'reactive_graph', 'tachys', 'throw_error']
        cargo_home = self.root / 'cargo'
        cache = cargo_home / 'registry/cache/fixture'
        cache.mkdir(parents=True)
        seed = {}
        lock = ''
        for name in names:
            archive = cache / f'{name}-1.0.0.crate'
            manifest = f'[package]\nname="{name}"\nversion="1.0.0"\nrepository="{vendor.REPO}"\nlicense="MIT"\n'
            with tarfile.open(archive, 'w:gz') as tar:
                for file, content in {'Cargo.toml': manifest, '.cargo_vcs_info.json': '{"git": {"sha1": "fixture", "dirty": true}}'}.items():
                    data = content.encode()
                    member = tarfile.TarInfo(f'{name}-1.0.0/{file}')
                    member.size = len(data)
                    tar.addfile(member, io.BytesIO(data))
            seed[name] = {'registry_source': 'registry+fixture', 'archive_sha256': vendor.sha(archive.read_bytes())}
            lock += f'[[package]]\nname="{name}"\nversion="1.0.0"\n'
        seed_file = self.root / 'seed.json'
        seed_file.write_text(json.dumps({'packages': seed}))
        (self.root / 'web/Cargo.lock').write_text(lock)
        output = self.root / 'reimport'
        with patch.multiple(vendor, DEST=output, PROVENANCE=output / 'provenance.json'), patch.dict(os.environ, {'CARGO_HOME': str(cargo_home)}), contextlib.redirect_stdout(io.StringIO()):
            vendor.import_archives(seed_file)
        imported = json.loads((output / 'provenance.json').read_text())['packages']
        self.assertEqual(set(imported), set(names))
        self.assertTrue(imported['tachys']['vcs']['git']['dirty'])
        self.assertEqual(imported['tachys']['archive_sha256'], seed['tachys']['archive_sha256'])


if __name__ == '__main__':
    unittest.main()
