#[test]
fn check_numassignref_ops() {
    fn compute<T: NumAssignRef + Copy>(mut x: T, y: &T) -> T {
        x *= y;
        x /= y;
        x %= y;
        x += y;
        x -= y;
        x
    }
    assert_eq!(compute(1, &2), 1)
}
