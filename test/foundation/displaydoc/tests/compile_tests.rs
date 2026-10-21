#[allow(unused_attributes)]
#[rustversion::attr(not(nightly), ignore)]
#[test]
fn no_std() {
    let t = trybuild::TestCases::new();
    t.compile_fail("../../../test/foundation/displaydoc/tests/ui/without.rs");
    t.compile_fail("../../../test/foundation/displaydoc/tests/ui/multi_line_line_break.rs");
    t.compile_fail("../../../test/foundation/displaydoc/tests/ui/multi_line_line_break_block.rs");
    t.pass("../../../test/foundation/displaydoc/tests/ui/multi_line.rs");
    t.pass("../../../test/foundation/displaydoc/tests/ui/multi_line_allow.rs");
    t.compile_fail("../../../test/foundation/displaydoc/tests/ui/enum_prefix_missing.rs");
    t.pass("../../../test/foundation/displaydoc/tests/ui/enum_prefix.rs");
    t.pass("../../../test/foundation/displaydoc/tests/ui/multiple.rs");
}
