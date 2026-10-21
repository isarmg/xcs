#[test]
fn test_set_bit() {
    use crate::consts::*;
    use crate::Same;
    type T1 = <SetBitOut<U2, U0, B0> as Same<U2>>::Output;
    type T2 = <SetBitOut<U2, U0, B1> as Same<U3>>::Output;
    type T3 = <SetBitOut<U2, U1, B0> as Same<U0>>::Output;
    type T4 = <SetBitOut<U2, U1, B1> as Same<U2>>::Output;
    type T5 = <SetBitOut<U2, U2, B0> as Same<U2>>::Output;
    type T6 = <SetBitOut<U2, U2, B1> as Same<U6>>::Output;
    type T7 = <SetBitOut<U2, U3, B0> as Same<U2>>::Output;
    type T8 = <SetBitOut<U2, U3, B1> as Same<U10>>::Output;
    type T9 = <SetBitOut<U2, U4, B0> as Same<U2>>::Output;
    type T10 = <SetBitOut<U2, U4, B1> as Same<U18>>::Output;

    type T11 = <SetBitOut<U3, U0, B0> as Same<U2>>::Output;

    <T1 as Unsigned>::to_u32();
    <T2 as Unsigned>::to_u32();
    <T3 as Unsigned>::to_u32();
    <T4 as Unsigned>::to_u32();
    <T5 as Unsigned>::to_u32();
    <T6 as Unsigned>::to_u32();
    <T7 as Unsigned>::to_u32();
    <T8 as Unsigned>::to_u32();
    <T9 as Unsigned>::to_u32();
    <T10 as Unsigned>::to_u32();
    <T11 as Unsigned>::to_u32();
}
