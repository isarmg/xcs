"""Historical reconstruction never hides missing or modified live source."""
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('historical_test_sources', Path(__file__).resolve().parents[3] / 'tools/rust-sources/historical_test_sources.py')
history = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(history)


class HistoricalReconstruction(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        archived = patch.object(history, "RETIREMENT_ARCHIVES", {})
        archived.start(); self.addCleanup(archived.stop)
        self.source = self.root / 'src/mini/lib.rs'
        self.source.parent.mkdir(parents=True)
        self.source.write_text('include!("tests.rs");')
        self.test = self.root / 'test/mini/tests.rs'
        self.test.parent.mkdir(parents=True)
        self.test.write_text('assert!(true);')
        before = 'mod tests { assert!(true); }'
        digest = lambda data: hashlib.sha256(data.encode()).hexdigest()
        record = {'schema_version': 1, 'roots': {'test/mini': {'test/mini/tests.rs': digest(self.test.read_text())}}, 'files': [{'source': 'src/mini/lib.rs', 'path': 'src/mini/lib.rs', 'sha256': digest(self.source.read_text()), 'before_sha256': digest(before), 'replacement': {'old': before, 'new': self.source.read_text()}}]}
        self.receipt = self.root / history.RECEIPT
        self.receipt.parent.mkdir(parents=True)
        self.receipt.write_text(json.dumps(record))
        patched = patch.object(history, 'RECEIPT_SHA256', hashlib.sha256(self.receipt.read_bytes()).hexdigest())
        patched.start(); self.addCleanup(patched.stop)
        self.corrections = self.root / history.CORRECTIONS
        self.save_corrections([])

    def save_corrections(self, rows):
        self.corrections.write_text(json.dumps({'schema_version': 1, 'files': rows}))
        patched = patch.object(history, 'CORRECTIONS_SHA256', hashlib.sha256(self.corrections.read_bytes()).hexdigest())
        patched.start(); self.addCleanup(patched.stop)

    def routing_fix(self):
        old = self.test.read_bytes()
        self.test.write_bytes(b'assert!(true); // routed')
        row = {'source': 'test/mini/tests.rs', 'path': 'test/mini/tests.rs',
               'sha256': hashlib.sha256(self.test.read_bytes()).hexdigest(),
               'before_sha256': hashlib.sha256(old).hexdigest(),
               'replacement': {'old': 'assert!(true);', 'new': 'assert!(true); // routed'}}
        self.save_corrections([row])
        return row

    def test_authenticated_routing_correction_preserves_original_receipt(self):
        original_receipt = self.receipt.read_bytes()
        self.routing_fix()
        self.assertEqual(history.historical_reader(self.root)('test/mini/tests.rs'), b'assert!(true);')
        self.assertEqual(self.receipt.read_bytes(), original_receipt)

    def test_correction_current_tamper_rejected(self):
        self.routing_fix()
        self.test.write_text('assert!(false); // routed')
        with self.assertRaisesRegex(ValueError, 'corrected source content'):
            history.historical_reader(self.root)

    def test_wrong_correction_and_receipt_tamper_rejected(self):
        row = self.routing_fix()
        row['replacement']['old'] = 'assert!(false);'
        self.save_corrections([row])
        with self.assertRaisesRegex(ValueError, 'historical content'):
            history.historical_reader(self.root)
        self.corrections.write_bytes(self.corrections.read_bytes() + b' ')
        with self.assertRaisesRegex(ValueError, 'correction receipt changed'):
            history.historical_reader(self.root)

    def test_exact_fixture_move_and_old_path_reappearance(self):
        path = self.root / 'test/mini/fixture'
        path.write_bytes(b'fixture')
        source = self.root / 'src/mini/fixture'
        sha = hashlib.sha256(path.read_bytes()).hexdigest()
        self.save_corrections([{'source': 'src/mini/fixture', 'path': 'test/mini/fixture',
                                'sha256': sha, 'before_sha256': sha}])
        self.assertEqual(history.historical_reader(self.root)('src/mini/fixture'), b'fixture')
        source.write_bytes(b'fixture')
        with self.assertRaisesRegex(ValueError, 'absent-source'):
            history.historical_reader(self.root)

    def test_exact_live_sources_reconstruct_prior_view(self):
        read = history.historical_reader(self.root)
        self.assertEqual(read('src/mini/lib.rs'), b'mod tests { assert!(true); }')

    def test_missing_changed_and_extra_external_material_rejected(self):
        for mutation in ('missing', 'changed', 'extra'):
            with self.subTest(mutation=mutation):
                original = self.test.read_bytes()
                extra = self.test.parent / 'extra.rs'
                if mutation == 'missing': self.test.unlink()
                elif mutation == 'changed': self.test.write_text('assert!(false);')
                else: extra.write_text('new')
                with self.assertRaises(ValueError): history.historical_reader(self.root)
                self.test.write_bytes(original); extra.unlink(missing_ok=True)

    def test_live_production_change_rejected(self):
        self.source.write_text('changed production')
        with self.assertRaisesRegex(ValueError, 'current source'): history.historical_reader(self.root)

    def test_receipt_change_rejected(self):
        self.receipt.write_bytes(self.receipt.read_bytes() + b' ')
        with self.assertRaisesRegex(ValueError, 'receipt changed'): history.historical_reader(self.root)

    def test_symlink_and_escape_rejected(self):
        self.test.unlink(); self.test.symlink_to(self.source)
        with self.assertRaisesRegex(ValueError, 'symlink'): history.historical_reader(self.root)
        self.test.unlink(); self.test.write_text('assert!(true);')
        read = history.historical_reader(self.root)
        with self.assertRaises(ValueError): read('../outside')


if __name__ == '__main__':
    unittest.main()
