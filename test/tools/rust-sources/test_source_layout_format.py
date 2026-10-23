"""Only the single reviewed maintained-test rustfmt wrap may change whitespace."""
import unittest
from test_source_layout import LayoutFixture,layout

class ExactReviewedFormatting(LayoutFixture):
    def test_exact_maintained_join_line_gets_only_reviewed_three_line_wrap(self):
        old='src/foundation/reactive-state';new='src/foundation/web-runtime/reactive-state'
        before=(b'// Preserve  unrelated   spaces.\n'
                b'    let dependency = fixture.0.join("src/foundation/reactive-state/src/lib.rs");\n'
                b'    assert!(dependency.exists());  // keep two spaces\n')
        after=(b'// Preserve  unrelated   spaces.\n'
               b'    let dependency = fixture\n        .0\n'
               b'        .join("src/foundation/web-runtime/reactive-state/src/lib.rs");\n'
               b'    assert!(dependency.exists());  // keep two spaces\n')
        self.assertEqual(layout.rewrite('test/tools/web-build/unit/main_tests.rs',before,{old:new}),after)

    def test_identical_line_in_unreviewed_file_does_not_get_format_exception(self):
        before=b'    let dependency = fixture.0.join("src/foundation/reactive-state/src/lib.rs");\n'
        with self.assertRaises(ValueError):
            layout.rewrite('test/tools/web-build/unit/not_reviewed.rs',before,
                           {'src/foundation/reactive-state':'src/foundation/web-runtime/reactive-state'})

    def test_self_consistent_unreviewed_whitespace_edit_still_rejected(self):
        path='src/foundation/plain/src/lib.rs';before=self.scenario.before_bytes[path]
        self.add_unapproved_change(path,before,before+b'\n')
        with self.assertRaises(ValueError):self.install()

if __name__=='__main__':unittest.main(verbosity=2)
