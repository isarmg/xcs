#[test]
fn fold_test() {
    use crate::*;
    assert_eq!(10, <FoldSum::<tarr![U2, U3, U5]>>::to_u32());
    assert_eq!(30, <FoldProd::<tarr![U2, U3, U5]>>::to_u32());
}
