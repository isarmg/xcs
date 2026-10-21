#[test]
fn test_rust_to_clang_target_simulator() {
    assert_eq!(
        rust_to_clang_target("aarch64-apple-ios-sim").as_ref(),
        "arm64-apple-ios-simulator"
    );
    assert_eq!(
        rust_to_clang_target("aarch64-apple-tvos-sim").as_ref(),
        "arm64-apple-tvos-simulator"
    );
    assert_eq!(
        rust_to_clang_target("aarch64-apple-watchos-sim").as_ref(),
        "arm64-apple-watchos-simulator"
    );
}
