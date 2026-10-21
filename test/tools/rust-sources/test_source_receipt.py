"""Bootstrap must preserve real source bytes, not only an attributed path label."""
import gzip
import hashlib
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / 'tools/rust-sources'))
import external_tests
SPEC = importlib.util.spec_from_file_location('source_receipt_checker', Path(__file__).resolve().parents[3] / 'tools/rust-sources/source_relocation.py')
receipt = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(receipt)


class BootstrapPreservation(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.source = 'src/mini/tests.rs'; self.destination = 'test/mini/tests.rs'
        self.file = self.root / self.destination; self.file.parent.mkdir(parents=True)
        self.file.write_bytes(b'assert!(true);\n')
        self.digest = hashlib.sha256(self.file.read_bytes()).hexdigest()
        self.owners = {'src/mini': {'files': [dict(source=self.source, path=self.destination, source_sha256_before=self.digest, sha256=self.digest)]}}
        self.record = dict(schema_version=1, mapping=[[self.source, self.destination, self.digest, self.digest]], sources={self.source: dict(before_sha256=self.digest, outputs=[dict(path=self.destination, sha256=self.digest)], reverse_edits=[])})
        self.path = self.root / receipt.RECEIPT; self.path.parent.mkdir(parents=True)
        self.path.write_bytes(gzip.compress(json.dumps(self.record).encode(), mtime=0))
        patched = patch.object(receipt, 'RECEIPT_SHA256', hashlib.sha256(self.path.read_bytes()).hexdigest())
        patched.start(); self.addCleanup(patched.stop)

    def verify(self):
        return receipt.verify(self.root, self.owners, external_tests.safe, {self.source: self.digest})

    def test_exact_move_reconstructs_original(self):
        self.verify()

    def test_empty_or_truncated_destination_cannot_be_blessed_by_mapping_hash(self):
        for content in (b'', b'assert!'):
            with self.subTest(content=content):
                self.file.write_bytes(content)
                self.owners['src/mini']['files'][0]['sha256'] = hashlib.sha256(content).hexdigest()
                with self.assertRaisesRegex(ValueError, 'mapping changed'): self.verify()

    def test_wrong_source_substitution_rejected(self):
        self.owners['src/mini']['files'][0]['source'] = 'src/mini/unrelated.rs'
        with self.assertRaisesRegex(ValueError, 'mapping changed'): self.verify()

    def test_live_bytes_checked_even_with_original_mapping(self):
        self.file.write_bytes(b'')
        with self.assertRaisesRegex(ValueError, 'current content changed'): self.verify()

    def test_prior_catalog_baseline_must_match(self):
        with self.assertRaisesRegex(ValueError, 'previous catalog'):
            receipt.verify(self.root, self.owners, external_tests.safe, {self.source: '0' * 64})

    def test_receipt_tampering_and_truncation_rejected(self):
        original = self.path.read_bytes()
        for data in (original[:-1], original + b'tampered'):
            with self.subTest(size=len(data)):
                self.path.write_bytes(data)
                with self.assertRaisesRegex(ValueError, 'receipt changed'): self.verify()

    def test_expansion_is_bounded(self):
        with patch.object(receipt, 'MAX_EXPANDED', 10):
            with self.assertRaisesRegex(ValueError, 'exceeds bound'): self.verify()

    def test_parent_symlink_rejected(self):
        self.file.unlink(); self.file.parent.rmdir()
        self.file.parent.symlink_to(self.path.parent, target_is_directory=True)
        with self.assertRaisesRegex(ValueError, 'symlink'): self.verify()


if __name__ == '__main__':
    unittest.main()
