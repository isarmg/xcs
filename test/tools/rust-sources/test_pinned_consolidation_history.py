"""Prior unrelated records must never replace pending pinned authorization."""
from pathlib import Path
import sys
import tempfile
import unittest

TOOLS = Path(__file__).resolve().parents[3] / 'tools/rust-sources'
sys.path.insert(0, str(TOOLS))
import source_consolidation


class PinnedConsolidationHistory(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        compliance = self.root / 'compliance'
        compliance.mkdir()
        (compliance / 'platform-test-relocation.json').write_text(
            '{"source":"src/foundation/parking-core/src/lib.rs"}'
        )
        self.directory = 'src/foundation/parking-core'
        self.inputs = {
            self.directory: {'origin_directory': 'original-parking'},
        }
        self.pending = {
            'primary_directory': self.directory,
            'destination_directory': 'src/foundation/synchronization',
        }
        self.history = {
            'old': {
                'inputs': [{'package': {'directory': 'src/foundation/old-core'}}],
                'destination_directory': 'src/foundation/old-merged',
            },
            'second': {
                'inputs': [{'package': {'directory': 'src/foundation/other-core'}}],
                'destination_directory': 'src/foundation/other-merged',
            },
        }

    def check(self, authorize):
        source_consolidation._unhandled_history(
            self.root,
            {},
            self.inputs,
            'src/foundation/synchronization',
            self.history,
            self.pending,
            authorize,
        )

    def test_unrelated_history_keeps_pending_record_identity(self):
        seen = []

        def authorize(root, previous, record):
            seen.append(record)
            return record is self.pending

        self.check(authorize)
        self.assertEqual(len(seen), 1)
        self.assertIs(seen[0], self.pending)

    def test_old_record_cannot_authorize_pending(self):
        with self.assertRaisesRegex(ValueError, 'historical-view adapter'):
            self.check(
                lambda root, previous, record: record is self.history['second']
            )

    def test_prior_overlapping_owner_is_still_rejected(self):
        self.history['old']['inputs'][0]['package']['directory'] = self.directory

        def unauthorized(*args):
            self.fail('authorization must not run after overlap')

        with self.assertRaisesRegex(ValueError, 'prior consolidation inputs'):
            self.check(unauthorized)


if __name__ == '__main__':
    unittest.main()
