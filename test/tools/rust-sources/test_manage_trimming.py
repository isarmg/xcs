"""Manager refresh/verify wiring for explicit per-file trimming (no Cargo)."""
from copy import deepcopy
import unittest

import test_manage as fixture


class ManageTrimming(unittest.TestCase):
    origin = fixture.SourcePolicy.origin
    write_json = fixture.SourcePolicy.write_json
    save_catalog = fixture.SourcePolicy.save_catalog
    refresh = fixture.SourcePolicy.refresh
    verify = fixture.SourcePolicy.verify

    def setUp(self):
        fixture.SourcePolicy.setUp(self)
        (self.source / 'build.rs').write_text('fn main() {}')
        self.refresh()
        directory = 'src/foundation/mini'
        before = deepcopy(self.catalog['packages'][directory])
        self.plan = {'schema': 1, 'packages': [{'directory': directory, 'package': before,
            'files': [{'path': 'build.rs', 'sha256_before': before['files_sha256']['build.rs'],
                       'reason': 'Remove unused build script'}]}]}
        (self.source / 'build.rs').unlink()

    def test_refresh_requires_plan_then_preserves_history(self):
        with self.assertRaisesRegex(ValueError, 'unaccounted source removal'):
            self.refresh()
        self.refresh(trim_plan_data=self.plan)
        self.verify()
        history = deepcopy(self.catalog['source_trims'])
        self.refresh()
        self.assertEqual(history, self.catalog['source_trims'])
        self.verify()

    def test_extra_removal_still_rejected(self):
        (self.source / 'lib.rs').unlink()
        with self.assertRaisesRegex(ValueError, 'unaccounted source removal'):
            self.refresh(trim_plan_data=self.plan)

    def test_trim_and_retirement_conflict(self):
        with self.assertRaisesRegex(ValueError, 'trimmed owner lifecycle'):
            self.refresh(trim_plan_data=self.plan, retire_plan={
                'schema': 1, 'packages': [{'directory': 'src/foundation/mini'}]})

    def test_restored_file_and_changed_receipt_fail_verify_and_refresh(self):
        self.refresh(trim_plan_data=self.plan)
        key = next(iter(self.catalog['source_trims']))
        self.catalog['source_trims'][key]['files'][0]['reason'] = 'changed after review'
        self.save_catalog()
        with self.assertRaisesRegex(ValueError, 'source trim receipt integrity'):
            self.verify()
        with self.assertRaisesRegex(ValueError, 'source trim receipt integrity'):
            self.refresh()


if __name__ == '__main__':
    unittest.main()
