#[test]
fn traits() {
    use crate::test_helpers::*;
    assert_send::<Router<()>>();
    assert_sync::<Router<()>>();
    assert_send::<RouterAsService<'static, Body, ()>>();
    assert_sync::<RouterAsService<'static, Body, ()>>();
    assert_send::<RouterIntoService<Body, ()>>();
    assert_sync::<RouterIntoService<Body, ()>>();
}
