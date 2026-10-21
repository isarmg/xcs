// Only test on stable, since UI tests are bound to change over time

#[rustversion::stable]
#[test]
fn pass() {
    let t = trybuild::TestCases::new();
    t.pass("../../../test/foundation/tracing-attributes/tests/ui/pass/*.rs");
}

#[rustversion::stable]
#[test]
fn compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("../../../test/foundation/tracing-attributes/tests/ui/fail/*.rs");
}
