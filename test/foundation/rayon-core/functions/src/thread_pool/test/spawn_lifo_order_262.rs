#[test]
#[cfg_attr(any(target_os = "emscripten", target_family = "wasm"), ignore)]
fn spawn_lifo_order() {
    let vec = test_spawn_order!(spawn);
    let expected: Vec<i32> = (0..10).rev().collect(); // LIFO -> reversed
    assert_eq!(vec, expected);
}
