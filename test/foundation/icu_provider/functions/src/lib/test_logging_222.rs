#[test]
fn test_logging() {
    // This should compile on all combinations of features
    crate::log::info!("Hello World");
}
