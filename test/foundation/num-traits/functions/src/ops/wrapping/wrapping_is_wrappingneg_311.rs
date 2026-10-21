#[test]
fn wrapping_is_wrappingneg() {
    fn require_wrappingneg<T: WrappingNeg>(_: &T) {}
    require_wrappingneg(&Wrapping(42));
}
