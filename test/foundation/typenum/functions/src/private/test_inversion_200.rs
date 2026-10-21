#[test]
fn test_inversion() {
    type Test4 = <crate::consts::U4 as Invert>::Output;
    type Test5 = <crate::consts::U5 as Invert>::Output;
    type Test12 = <crate::consts::U12 as Invert>::Output;
    type Test16 = <crate::consts::U16 as Invert>::Output;

    assert_eq!(1, <Test4 as InvertedUnsigned>::to_u64());
    assert_eq!(5, <Test5 as InvertedUnsigned>::to_u64());
    assert_eq!(3, <Test12 as InvertedUnsigned>::to_u64());
    assert_eq!(1, <Test16 as InvertedUnsigned>::to_u64());
}
