// SPDX-License-Identifier: Apache-2.0 OR MIT

#![cfg(not(miri))]

#[rustversion::attr(not(nightly), ignore)]
#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("../../../test/foundation/pin-project-lite/tests/ui/**/*.rs");
    t.pass("../../../test/foundation/pin-project-lite/tests/run-pass/**/*.rs");
}
