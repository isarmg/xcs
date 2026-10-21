#[test]
fn test_rust_to_clang_target() {
    assert_eq!(
        rust_to_clang_target("aarch64-apple-ios").as_ref(),
        "arm64-apple-ios"
    );
}
