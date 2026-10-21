#[rustversion::attr(not(nightly), ignore = "requires nightly")]
#[cfg_attr(miri, ignore = "incompatible with miri")]
#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("../../../test/foundation/quote/tests/ui/*.rs");
}
