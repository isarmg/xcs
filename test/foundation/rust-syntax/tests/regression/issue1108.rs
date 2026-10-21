#[test]
fn issue1108() {
    let data = "impl<x<>>::x for";
    let _ = rust_syntax::parse_file(data);
}
