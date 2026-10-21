#[test]
fn wrapping_is_wrappingsub() {
    fn require_wrappingsub<T: WrappingSub>(_: &T) {}
    require_wrappingsub(&Wrapping(42));
}
