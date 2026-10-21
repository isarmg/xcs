#[test]
fn broadcast_global() {
    let v = crate::broadcast(|ctx| ctx.index());
    assert!(v.into_iter().eq(0..crate::current_num_threads()));
}
