#[test]
fn traits() {
    use crate::test_helpers::*;
    assert_send::<IntoMakeServiceWithConnectInfo<(), NotSendSync>>();
}
