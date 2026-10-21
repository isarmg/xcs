"""Synthetic checks that ring proof history never replaces current execution inputs."""
import contextlib
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock

SPEC = importlib.util.spec_from_file_location(
    'ring_live_routing_checker',
    Path(__file__).resolve().parents[3] / 'tools/rust-sources/check_ring_tests.py',
)
checker = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(checker)


class RoutedView:
    """Mock only the already-authenticated routing API, not current file reads."""
    def __init__(self, root, renamed=True):
        self.root = root
        self.directory = 'src/foundation/ring-next' if renamed else str(checker.RING)
        self.test_root = 'test/foundation/ring-next' if renamed else 'test/foundation/ring'
        self.name = 'ring-next' if renamed else 'ring'
        self.historical = {}

    def current_package(self, directory):
        assert directory == str(checker.RING)
        return dict(directory=self.directory, name=self.name, version='0.17.14')

    def current_path(self, path):
        path = str(path)
        for old, new in ((str(checker.RING), self.directory), ('test/foundation/ring', self.test_root)):
            if path == old or path.startswith(old + '/'):
                return new + path[len(old):]
        return path

    def baseline_path(self, path):
        for old, new in ((str(checker.RING), self.directory), ('test/foundation/ring', self.test_root)):
            if path == new or path.startswith(new + '/'):
                return old + path[len(new):]
        return path

    def read(self, path):
        if path in self.historical:
            return self.historical[path]
        return (self.root / self.current_path(path)).read_bytes()

    def inventory(self, base):
        return {self.baseline_path(path.relative_to(self.root).as_posix()):
                hashlib.sha256(self.read(self.baseline_path(path.relative_to(self.root).as_posix()))).hexdigest()
                for path in (self.root / self.current_path(base)).rglob('*') if path.is_file()}


class RingLiveRouting(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.view = RoutedView(self.root)
        self.proof = {'test_sources': [{'path': 'tests/a.rs'}]}
        self.make_workspace()
        patch = mock.patch.object(checker.proof_routing, 'load', return_value=self.view)
        self.load = patch.start()
        self.addCleanup(patch.stop)

    def write(self, relative, data):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data if isinstance(data, bytes) else data.encode())
        return path

    def make_workspace(self):
        directory, tests = self.view.directory, self.view.test_root
        manifest = f'[package]\nname = "{self.view.name}"\nversion = "0.17.14"\n'
        for name, relative in (('a', 'tests/a.rs'), ('shavs', 'cavp/tests/shavs.rs')):
            self.write(tests + '/' + relative, '// current test\n')
            target = os.path.relpath(self.root / tests / relative, self.root / directory)
            manifest += f'\n[[test]]\nname = "{name}"\npath = "{target}"\n'
        self.manifest = self.write(directory + '/Cargo.toml', manifest)
        self.write('Cargo.toml', f'[workspace]\nmembers = ["{directory}"]\ndefault-members = []\n')

    def generated_fixture(self, output_name=None):
        source = self.root / self.view.directory / 'pregenerated'
        output = self.root / 'target/debug/build' / (output_name or self.view.name + '-test') / 'out'
        for i in range(12):
            relative = f'current-{i}-x86_64-elf.S'
            self.write(str(source.relative_to(self.root) / relative), f'current assembly {i}\n')
            self.write(str(output.relative_to(self.root) / relative), f'current assembly {i}\n')
        for name in ('prefix_symbols.h', 'prefix_symbols_asm.h', 'prefix_symbols_nasm.inc'):
            relative = Path('ring_core_generated') / name
            self.write(str(source.relative_to(self.root) / relative), 'current package prefix\n')
            self.write(str(output.relative_to(self.root) / relative), 'current package prefix\n')
        return source, output

    def test_renamed_live_manifest_and_inverse_mapped_targets_verify(self):
        self.assertFalse((self.root / checker.RING).exists())
        self.assertEqual(checker.verify_current_targets(self.root, self.proof),
                         {'tests/a.rs', 'cavp/tests/shavs.rs'})

    def test_same_root_cargo_alias_edit_uses_actual_manifest(self):
        self.view = RoutedView(self.root, renamed=False)
        self.load.return_value = self.view
        self.make_workspace()
        self.manifest.write_text(self.manifest.read_text() + '\n[dependencies]\nrenamed-helper = { package = "helper", version = "1" }\n')
        self.view.historical[str(checker.RING / 'Cargo.toml')] = b'invalid historical manifest'
        checker.verify_current_targets(self.root, self.proof)
        _, _, manifest = checker.current_ring(self.root)
        self.assertIn('renamed-helper', manifest['dependencies'])

    def test_missing_current_manifest_cannot_use_historical_bytes(self):
        self.view.historical[str(checker.RING / 'Cargo.toml')] = self.manifest.read_bytes()
        self.manifest.unlink()
        with self.assertRaisesRegex(ValueError, 'current ring manifest is missing'):
            checker.verify_current_targets(self.root, self.proof)

    def test_missing_current_target_cannot_use_historical_bytes(self):
        relative = 'test/foundation/ring/tests/a.rs'
        self.view.historical[relative] = b'// historical test\n'
        (self.root / self.view.current_path(relative)).unlink()
        with self.assertRaisesRegex(ValueError, 'current ring test target is missing'):
            checker.verify_current_targets(self.root, self.proof)

    def test_target_with_unmapped_or_incorrect_inverse_path_is_rejected(self):
        with mock.patch.object(self.view, 'baseline_path', return_value='test/other/tests/a.rs'):
            with self.assertRaisesRegex(ValueError, 'outside its authenticated roots'):
                checker.verify_current_targets(self.root, self.proof)
        with mock.patch.object(self.view, 'baseline_path', return_value='test/foundation/ring/tests/wrong.rs'):
            with self.assertRaisesRegex(ValueError, 'entry set changed'):
                checker.verify_current_targets(self.root, self.proof)

    def test_current_target_cannot_hide_symlink_traversal_with_parent_components(self):
        directory = self.root / self.view.directory
        (directory / 'shortcut').symlink_to(directory, target_is_directory=True)
        self.manifest.write_text(self.manifest.read_text().replace('../../../test/', 'shortcut/../../../../test/'))
        with self.assertRaisesRegex(ValueError, 'traverses a symlink'):
            checker.verify_current_targets(self.root, self.proof)

    def test_live_workspace_must_name_current_not_historical_root(self):
        for members, defaults in (([str(checker.RING)], []), ([self.view.directory], [self.view.directory])):
            self.write('Cargo.toml', '[workspace]\nmembers = ' + json.dumps(members) + '\ndefault-members = ' + json.dumps(defaults) + '\n')
            with self.assertRaisesRegex(ValueError, 'explicit non-default'):
                checker.verify_current_targets(self.root, self.proof)

    def test_actual_matrix_command_selects_current_package(self):
        with (mock.patch.object(checker, 'verify') as verify,
              mock.patch.object(checker.subprocess, 'run') as run,
              mock.patch.object(checker, 'verify_generated') as generated,
              contextlib.redirect_stdout(io.StringIO())):
            checker.run_case(self.root, 'localgen', self.root / 'target', 'release', 'std,slow_tests')
        verify.assert_called_once_with(self.root)
        self.assertEqual(run.call_args.args[0], ['cargo', 'test', '--locked', '--offline', '-j2', '-p',
                                               'ring-next', '--release', '--features', 'std,slow_tests'])
        self.assertEqual(run.call_args.kwargs['cwd'], self.root)
        self.assertEqual(run.call_args.kwargs['env']['CARGO_TARGET_DIR'], str(self.root / 'target/localgen'))
        self.assertTrue(run.call_args.kwargs['check'])
        generated.assert_called_once_with(self.root, self.root / 'target/localgen', 'release')

    def test_generated_comparison_reads_current_root_and_current_build_prefix(self):
        self.generated_fixture()
        with mock.patch.object(self.view, 'read', side_effect=AssertionError('historical read used for execution')):
            with contextlib.redirect_stdout(io.StringIO()):
                checker.verify_generated(self.root, self.root / 'target', 'debug')

    def test_historical_package_build_output_cannot_satisfy_renamed_build(self):
        self.generated_fixture('ring-stale')
        with self.assertRaisesRegex(ValueError, 'no ring build output'):
            checker.verify_generated(self.root, self.root / 'target', 'debug')

    def test_generated_bytes_cannot_match_only_reconstructed_historical_source(self):
        source, output = self.generated_fixture()
        relative = Path('ring_core_generated/prefix_symbols.h')
        (output / relative).write_bytes(b'historical package prefix\n')
        self.view.historical[str(checker.RING / 'pregenerated' / relative)] = b'historical package prefix\n'
        with self.assertRaisesRegex(ValueError, 'differs from original package'):
            checker.verify_generated(self.root, self.root / 'target', 'debug')
        self.assertEqual((source / relative).read_bytes(), b'current package prefix\n')

    def test_missing_current_pregenerated_file_is_not_recovered_from_history(self):
        source, _ = self.generated_fixture()
        (source / 'current-0-x86_64-elf.S').unlink()
        with self.assertRaisesRegex(ValueError, 'generated-source inventory'):
            checker.verify_generated(self.root, self.root / 'target', 'debug')

    def test_copy_workspace_adds_development_marker_only_under_current_root(self):
        destination = self.root / 'isolated'
        destination.mkdir()
        paths = sorted(path.relative_to(self.root).as_posix() for path in self.root.rglob('*') if path.is_file())
        with (mock.patch.object(checker, 'ROOT', self.root),
              mock.patch.object(checker.subprocess, 'check_output', return_value='\0'.join(paths).encode() + b'\0')):
            checker.copy_workspace(destination)
        self.assertTrue((destination / self.view.directory / '.git').is_dir())
        self.assertFalse((destination / checker.RING).exists())

    def test_packaged_mode_rejects_development_marker_under_current_root(self):
        (self.root / self.view.directory / '.git').mkdir()
        with (mock.patch.object(checker, 'ROOT', self.root),
              mock.patch.object(checker, 'verify'),
              mock.patch.object(checker, 'run_matrix') as matrix,
              mock.patch.object(checker.platform, 'system', return_value='Linux'),
              mock.patch.object(checker.platform, 'machine', return_value='x86_64'),
              mock.patch.object(checker.subprocess, 'check_output', return_value='host: x86_64-unknown-linux-gnu'),
              mock.patch.dict(checker.os.environ, {}, clear=True),
              mock.patch('sys.argv', ['check_ring_tests.py', '--mode', 'packaged'])):
            with self.assertRaisesRegex(SystemExit, 'Packaged mode requires no'):
                checker.main()
        matrix.assert_not_called()

    def test_old_receipt_reads_baseline_inventory_after_active_routing(self):
        self.view.historical[str(checker.RING / 'Cargo.toml')] = self.manifest.read_bytes().replace(b'"ring-next"', b'"ring"')
        current = {path: self.view.read(path) for base in (str(checker.RING), 'test/foundation/ring')
                   for path in self.view.inventory(base)}
        rows = []
        for path, data in current.items():
            source = (str(checker.RING) + path[len('test/foundation/ring'):] if path.startswith('test/') else path)
            rows.append({'path': path, 'source': source, 'sha256': hashlib.sha256(data).hexdigest()})
        receipt = {'schema_version': 1, 'files': rows, 'external_files': {}}
        content = json.dumps(receipt).encode()
        self.write(str(checker.RELOCATION), content)
        with mock.patch.object(checker, 'RELOCATION_SHA256', hashlib.sha256(content).hexdigest()):
            historical = checker.relocated_sources(self.root)
        self.assertEqual(historical[str(checker.RING / 'tests/a.rs')], b'// current test\n')
        self.assertIn(b'name = "ring"', historical[str(checker.RING / 'Cargo.toml')])
        self.assertIn(b'name = "ring-next"', self.manifest.read_bytes())
        self.assertFalse((self.root / checker.RING).exists())


if __name__ == '__main__':
    unittest.main()
