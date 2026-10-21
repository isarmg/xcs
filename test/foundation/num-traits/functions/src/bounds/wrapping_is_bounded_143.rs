#[test]
fn wrapping_is_bounded() {
    fn require_bounded<T: Bounded>(_: &T) {}
    require_bounded(&Wrapping(42_u32));
    require_bounded(&Wrapping(-42));
}
