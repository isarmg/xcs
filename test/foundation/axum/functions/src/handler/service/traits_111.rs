#[test]
fn traits() {
    use crate::test_helpers::*;
    assert_send::<HandlerService<(), NotSendSync, ()>>();
    assert_sync::<HandlerService<(), NotSendSync, ()>>();
}
