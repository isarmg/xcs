#[test]
fn wrapping_is_wrappingshr() {
    fn require_wrappingshr<T: WrappingShr>(_: &T) {}
    require_wrappingshr(&Wrapping(42));
}
