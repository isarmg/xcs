#[test]
fn traits() {
    use crate::test_helpers::*;

    assert_send::<HandleError<(), (), NotSendSync>>();
    assert_sync::<HandleError<(), (), NotSendSync>>();
}
