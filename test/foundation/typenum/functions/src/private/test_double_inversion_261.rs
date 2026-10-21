#[test]
fn test_double_inversion() {
    type Test4 = <<crate::consts::U4 as Invert>::Output as Invert>::Output;
    type Test5 = <<crate::consts::U5 as Invert>::Output as Invert>::Output;
    type Test12 = <<crate::consts::U12 as Invert>::Output as Invert>::Output;
    type Test16 = <<crate::consts::U16 as Invert>::Output as Invert>::Output;

    assert_eq!(4, <Test4 as Unsigned>::to_u64());
    assert_eq!(5, <Test5 as Unsigned>::to_u64());
    assert_eq!(12, <Test12 as Unsigned>::to_u64());
    assert_eq!(16, <Test16 as Unsigned>::to_u64());
}
