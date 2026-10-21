#[cfg(feature = "std")]
#[test]
fn it_works_with_to_string() {
    let x = errno();
    let _ = x.to_string();
}
