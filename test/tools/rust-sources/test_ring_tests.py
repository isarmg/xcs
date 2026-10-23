"""Offline guards for authenticated ring regression material and exact test entries."""
import contextlib
from copy import deepcopy
import importlib.util
import io
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest import mock

SPEC = importlib.util.spec_from_file_location('ring_test_checker', (Path(__file__).resolve().parents[3] / "tools/rust-sources" / 'check_ring_tests.py'))
checker = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(checker)


class RingMaterials(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # Proof/style mutations never change repository files. Authenticate the
        # unchanged source inputs once, and copy before each byte mutation.
        layout = checker.source_layout.load(checker.ROOT)
        with checker.source_layout.activate(layout):
            cls.historical_fixture = staticmethod(checker.historical_sources(checker.ROOT))
            cls.relocated_fixture = checker.relocated_sources(checker.ROOT)

    def verify(self):
        with contextlib.redirect_stdout(io.StringIO()):
            return checker.verify()

    def test_complete_pinned_proof_verifies_offline(self):
        proof = self.verify()
        self.assertEqual(len(proof['materials']), 69)
        self.assertEqual(len(proof['implementation_sources']), 380)
        self.assertTrue(proof['original_package_dirty'])
        self.assertIn('Unsigned', proof['commit_verification'])

    def mutate_proof(self, mutate, error):
        historical = self.historical_fixture
        proof = json.loads((checker.ROOT / checker.PROOF).read_text())
        mutate(proof)
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'proof.json'
            path.write_text(json.dumps(proof))
            with mock.patch.object(checker, 'PROOF', path), mock.patch.object(checker, 'historical_sources', return_value=historical):
                with self.assertRaisesRegex(ValueError, error):
                    self.verify()

    def mutate_style(self, mutate, error):
        record = json.loads((checker.ROOT / checker.STYLE).read_text())
        mutate(record)
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'style.json'
            path.write_text(json.dumps(record))
            with mock.patch.object(checker, 'STYLE', path):
                with self.assertRaisesRegex(ValueError, error):
                    self.verify()

    def test_style_patches_reconstruct_original_sources(self):
        historical = self.historical_fixture
        # This test only reads sources; authenticate one local snapshot, not one per patch.
        relocated = self.relocated_fixture
        for patch in json.loads((checker.ROOT / checker.STYLE).read_text())['patches']:
            before = historical(checker.RING / patch['path'])
            current = relocated[str(checker.RING / patch['path'])]
            self.assertNotEqual(before, current)
            self.assertEqual(before.replace(patch['old'].encode(), patch['new'].encode()), current)

    def test_style_rejects_extra_source(self):
        self.mutate_style(lambda r: r['patches'].append(r['patches'][0]), 'source set changed')

    def test_style_rejects_arbitrary_transformation(self):
        self.mutate_style(lambda r: r['patches'][0].update(new='unsafe {}'), 'unapproved style')

    def test_style_rejects_before_and_after_hash_drift(self):
        for label in ('before', 'after'):
            with self.subTest(label=label):
                self.mutate_style(lambda r: r['patches'][0].update({label + '_sha256': '0' * 64}), 'hash mismatch')

    def test_style_rejects_unrecorded_current_source_change(self):
        current = deepcopy(self.relocated_fixture)
        current[str(checker.RING / 'src/testutil.rs')] += b'// unrecorded\n'
        with mock.patch.object(checker, 'relocated_sources', return_value=current):
            with self.assertRaisesRegex(ValueError, 'hash mismatch'):
                self.verify()

    def test_style_rejects_historical_proof_drift(self):
        self.mutate_style(lambda r: r['historical_proofs'].update({str(checker.PROOF): '0' * 64}), 'immutable historical proof')

    def test_rejects_commit_drift(self):
        self.mutate_proof(lambda p: p.update(commit='0' * 40), 'unexpected upstream')

    def test_rejects_commit_object_drift(self):
        self.mutate_proof(lambda p: p.update(commit_object=p['commit_object'] + 'x'), 'commit object')

    def test_rejects_tree_drift(self):
        self.mutate_proof(lambda p: p['git_tree_entries'][0].update(sha='0' * 40), 'Git tree mismatch')

    def test_rejects_material_hash_drift(self):
        self.mutate_proof(lambda p: p['materials'][0].update(sha256='0' * 64), 'material mismatch')

    def test_rejects_missing_associated_source(self):
        self.mutate_proof(lambda p: p['materials'][0].update(associated_sources=[]), 'no associated test source')

    def test_rejects_test_source_hash_drift(self):
        self.mutate_proof(lambda p: p['materials'][0]['associated_sources'][0].update(sha256='0' * 64), 'associated test source mismatch')

    def test_rejects_implementation_hash_drift(self):
        self.mutate_proof(lambda p: p['implementation_sources'][0].update(sha256='0' * 64), 'changed an existing implementation')

    def test_rejects_duplicate_material(self):
        self.mutate_proof(lambda p: p['materials'].append(p['materials'][0]), 'duplicate material')

    def test_rejects_unsafe_paths(self):
        for path in ('../outside', '/outside', 'a/../b'):
            with self.subTest(path=path), self.assertRaises(ValueError):
                checker.checked_path(checker.ROOT, path)

    def test_matrix_has_exact_independent_modes(self):
        with mock.patch.object(checker, 'run_case') as run:
            checker.run_matrix(checker.ROOT, 'packaged', Path('/tmp/unused-ring-target'))
        self.assertEqual([call.args[3:] for call in run.call_args_list], [
            ('debug', 'default'), ('debug', 'std'), ('release', 'default'),
            ('release', 'std'), ('release', 'std,slow_tests'),
        ])

    def test_each_execution_is_verified_locked_offline_and_unfiltered(self):
        with mock.patch.object(checker, 'verify') as verify, mock.patch.object(checker.subprocess, 'run') as run, contextlib.redirect_stdout(io.StringIO()):
            checker.run_case(checker.ROOT, 'packaged', Path('/tmp/unused-ring-target'), 'release', 'std,slow_tests')
        verify.assert_called_once_with(checker.ROOT)
        self.assertEqual(run.call_args.args[0], ['cargo', 'test', '--locked', '--offline', '-j2', '-p', 'ring', '--release', '--features', 'std,slow_tests'])
        self.assertTrue(run.call_args.kwargs['check'])

    def test_local_workspace_uses_build_filesystem_not_small_tmpfs(self):
        with tempfile.TemporaryDirectory() as temporary:
            with (mock.patch.object(checker, 'verify'),
                  mock.patch.object(checker, 'copy_workspace'),
                  mock.patch.object(checker, 'run_matrix'),
                  mock.patch.object(checker.platform, 'system', return_value='Linux'),
                  mock.patch.object(checker.platform, 'machine', return_value='x86_64'),
                  mock.patch.object(checker.subprocess, 'check_output', return_value='host: x86_64-unknown-linux-gnu'),
                  mock.patch.dict(checker.os.environ, {'CARGO_TARGET_DIR': temporary}, clear=True),
                  mock.patch('sys.argv', ['check_ring_tests.py', '--mode', 'localgen']),
                  mock.patch.object(checker.tempfile, 'TemporaryDirectory') as workspace):
                workspace.return_value.__enter__.return_value = str(Path(temporary) / 'copy')
                checker.main()
                self.assertEqual(workspace.call_args.kwargs['dir'], Path(temporary) / 'ring-regression')

    def test_local_generator_outputs_match_all_twelve_elf_sources_and_headers(self):
        with tempfile.TemporaryDirectory() as temporary:
            target = Path(temporary)
            output = target / 'debug/build/ring-test/out'
            _, package, _ = checker.current_ring(checker.ROOT)
            source = checker.ROOT / package['directory'] / 'pregenerated'
            output.mkdir(parents=True)
            for path in source.glob('*x86_64*elf.S'):
                shutil.copyfile(path, output / path.name)
            shutil.copytree(source / 'ring_core_generated', output / 'ring_core_generated')
            with contextlib.redirect_stdout(io.StringIO()):
                checker.verify_generated(checker.ROOT, target, 'debug')
            (output / 'x86_64-mont5-elf.S').write_text('corrupt')
            with self.assertRaisesRegex(ValueError, 'differs from original package'):
                checker.verify_generated(checker.ROOT, target, 'debug')

    def test_local_generator_must_have_real_build_output(self):
        with tempfile.TemporaryDirectory() as temporary, self.assertRaisesRegex(ValueError, 'no ring build output'):
            checker.verify_generated(checker.ROOT, Path(temporary), 'debug')


if __name__ == '__main__':
    unittest.main()
