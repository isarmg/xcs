#[test]
fn traits() {
    use crate::test_helpers::*;
    assert_send::<FromExtractor<(), NotSendSync, ()>>();
    assert_sync::<FromExtractor<(), NotSendSync, ()>>();
}
