#[test]
#[cfg(not(miri))]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("../../../test/foundation/lazy_static/tests/compile_fail/*.rs");
}
