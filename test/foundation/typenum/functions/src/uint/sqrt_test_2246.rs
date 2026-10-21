#[test]
fn sqrt_test() {
    use crate::consts::*;

    assert_eq!(0, <Sqrt<U0>>::to_u32());

    assert_eq!(1, <Sqrt<U1>>::to_u32());
    assert_eq!(1, <Sqrt<U2>>::to_u32());
    assert_eq!(1, <Sqrt<U3>>::to_u32());

    assert_eq!(2, <Sqrt<U4>>::to_u32());
    assert_eq!(2, <Sqrt<U5>>::to_u32());
    assert_eq!(2, <Sqrt<U6>>::to_u32());
    assert_eq!(2, <Sqrt<U7>>::to_u32());
    assert_eq!(2, <Sqrt<U8>>::to_u32());

    assert_eq!(3, <Sqrt<U9>>::to_u32());
    assert_eq!(3, <Sqrt<U10>>::to_u32());
    assert_eq!(3, <Sqrt<U11>>::to_u32());
    assert_eq!(3, <Sqrt<U12>>::to_u32());
    assert_eq!(3, <Sqrt<U13>>::to_u32());
    assert_eq!(3, <Sqrt<U14>>::to_u32());
    assert_eq!(3, <Sqrt<U15>>::to_u32());

    assert_eq!(4, <Sqrt<U16>>::to_u32());
    assert_eq!(4, <Sqrt<U17>>::to_u32());
    assert_eq!(4, <Sqrt<U18>>::to_u32());
    assert_eq!(4, <Sqrt<U19>>::to_u32());
    assert_eq!(4, <Sqrt<U20>>::to_u32());
    assert_eq!(4, <Sqrt<U21>>::to_u32());
    assert_eq!(4, <Sqrt<U22>>::to_u32());
    assert_eq!(4, <Sqrt<U23>>::to_u32());
    assert_eq!(4, <Sqrt<U24>>::to_u32());

    assert_eq!(5, <Sqrt<U25>>::to_u32());
    assert_eq!(5, <Sqrt<U26>>::to_u32());
    // ...
}
