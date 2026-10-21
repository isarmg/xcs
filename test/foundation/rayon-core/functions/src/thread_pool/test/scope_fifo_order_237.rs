#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn scope_fifo_order() {
    let vec = test_scope_order!(scope_fifo => spawn_fifo);
    let expected: Vec<i32> = (0..10).collect(); // FIFO -> natural order
    assert_eq!(vec, expected);
}
