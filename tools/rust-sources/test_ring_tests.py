"""Offline guards for authenticated ring regression material and exact test entries."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest import mock

SPEC = importlib.util.spec_from_file_location('ring_test_checker', Path(__file__).with_name('check_ring_tests.py'))
checker = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(checker)


class RingMaterials(unittest.TestCase):
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
        proof = json.loads((checker.ROOT / checker.PROOF).read_text())
        mutate(proof)
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'proof.json'
            path.write_text(json.dumps(proof))
            with mock.patch.object(checker, 'PROOF', path):
                with self.assertRaisesRegex(ValueError, error):
                    self.verify()

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
            source = checker.ROOT / checker.RING / 'pregenerated'
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
