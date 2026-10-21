#[cfg(test)]
mod test {
    use crate::prelude::Fraction;
    use ::numeric::collection::{One, Zero};
    use std::convert::TryInto;

    #[test]
    fn fraction_to_i() {
        let n_inf = Fraction::neg_infinity();
        let n_one = Fraction::one() * -1;
        let n_half = n_one / 2;
        let zero = Fraction::zero();
        let p_one = Fraction::one();
        let p_half = p_one / 2;
        let p_inf = Fraction::infinity();
        let nan = Fraction::nan();

        assert_eq!(n_inf.try_into() as Result<i8, ()>, Err(()));
        assert_eq!(n_inf.try_into() as Result<i16, ()>, Err(()));
        assert_eq!(n_inf.try_into() as Result<i32, ()>, Err(()));
        assert_eq!(n_inf.try_into() as Result<i64, ()>, Err(()));
        assert_eq!(n_inf.try_into() as Result<i128, ()>, Err(()));
        assert_eq!(n_inf.try_into() as Result<isize, ()>, Err(()));

        assert_eq!(n_one.try_into(), Ok(-1i8));
        assert_eq!(n_one.try_into(), Ok(-1i16));
        assert_eq!(n_one.try_into(), Ok(-1i32));
        assert_eq!(n_one.try_into(), Ok(-1i64));
        assert_eq!(n_one.try_into(), Ok(-1i128));
        assert_eq!(n_one.try_into(), Ok(-1isize));

        assert_eq!(n_half.try_into() as Result<i8, ()>, Err(()));
        assert_eq!(n_half.try_into() as Result<i16, ()>, Err(()));
        assert_eq!(n_half.try_into() as Result<i32, ()>, Err(()));
        assert_eq!(n_half.try_into() as Result<i64, ()>, Err(()));
        assert_eq!(n_half.try_into() as Result<i128, ()>, Err(()));
        assert_eq!(n_half.try_into() as Result<isize, ()>, Err(()));

        assert_eq!(zero.try_into(), Ok(0i8));
        assert_eq!(zero.try_into(), Ok(0i16));
        assert_eq!(zero.try_into(), Ok(0i32));
        assert_eq!(zero.try_into(), Ok(0i64));
        assert_eq!(zero.try_into(), Ok(0i128));
        assert_eq!(zero.try_into(), Ok(0isize));

        assert_eq!(p_half.try_into() as Result<i8, ()>, Err(()));
        assert_eq!(p_half.try_into() as Result<i16, ()>, Err(()));
        assert_eq!(p_half.try_into() as Result<i32, ()>, Err(()));
        assert_eq!(p_half.try_into() as Result<i64, ()>, Err(()));
        assert_eq!(p_half.try_into() as Result<i128, ()>, Err(()));
        assert_eq!(p_half.try_into() as Result<isize, ()>, Err(()));

        assert_eq!(p_one.try_into(), Ok(1i8));
        assert_eq!(p_one.try_into(), Ok(1i16));
        assert_eq!(p_one.try_into(), Ok(1i32));
        assert_eq!(p_one.try_into(), Ok(1i64));
        assert_eq!(p_one.try_into(), Ok(1i128));
        assert_eq!(p_one.try_into(), Ok(1isize));

        assert_eq!(p_inf.try_into() as Result<i8, ()>, Err(()));
        assert_eq!(p_inf.try_into() as Result<i16, ()>, Err(()));
        assert_eq!(p_inf.try_into() as Result<i32, ()>, Err(()));
        assert_eq!(p_inf.try_into() as Result<i64, ()>, Err(()));
        assert_eq!(p_inf.try_into() as Result<i128, ()>, Err(()));
        assert_eq!(p_inf.try_into() as Result<isize, ()>, Err(()));

        assert_eq!(nan.try_into() as Result<i8, ()>, Err(()));
        assert_eq!(nan.try_into() as Result<i16, ()>, Err(()));
        assert_eq!(nan.try_into() as Result<i32, ()>, Err(()));
        assert_eq!(nan.try_into() as Result<i64, ()>, Err(()));
        assert_eq!(nan.try_into() as Result<i128, ()>, Err(()));
        assert_eq!(nan.try_into() as Result<isize, ()>, Err(()));
    }

    #[test]
    fn fraction_to_u() {
        let n_inf = Fraction::neg_infinity();
        let n_one = Fraction::one() * -1;
        let n_half = n_one / 2;
        let zero = Fraction::zero();
        let p_one = Fraction::one();
        let p_half = p_one / 2;
        let p_inf = Fraction::infinity();
        let nan = Fraction::nan();

        assert_eq!(n_inf.try_into() as Result<u8, ()>, Err(()));
        assert_eq!(n_inf.try_into() as Result<u16, ()>, Err(()));
        assert_eq!(n_inf.try_into() as Result<u32, ()>, Err(()));
        assert_eq!(n_inf.try_into() as Result<u64, ()>, Err(()));
        assert_eq!(n_inf.try_into() as Result<u128, ()>, Err(()));
        assert_eq!(n_inf.try_into() as Result<usize, ()>, Err(()));

        assert_eq!(n_one.try_into() as Result<u8, ()>, Err(()));
        assert_eq!(n_one.try_into() as Result<u16, ()>, Err(()));
        assert_eq!(n_one.try_into() as Result<u32, ()>, Err(()));
        assert_eq!(n_one.try_into() as Result<u64, ()>, Err(()));
        assert_eq!(n_one.try_into() as Result<u128, ()>, Err(()));
        assert_eq!(n_one.try_into() as Result<usize, ()>, Err(()));

        assert_eq!(n_half.try_into() as Result<u8, ()>, Err(()));
        assert_eq!(n_half.try_into() as Result<u16, ()>, Err(()));
        assert_eq!(n_half.try_into() as Result<u32, ()>, Err(()));
        assert_eq!(n_half.try_into() as Result<u64, ()>, Err(()));
        assert_eq!(n_half.try_into() as Result<u128, ()>, Err(()));
        assert_eq!(n_half.try_into() as Result<usize, ()>, Err(()));

        assert_eq!(zero.try_into(), Ok(0u8));
        assert_eq!(zero.try_into(), Ok(0u16));
        assert_eq!(zero.try_into(), Ok(0u32));
        assert_eq!(zero.try_into(), Ok(0u64));
        assert_eq!(zero.try_into(), Ok(0u128));
        assert_eq!(zero.try_into(), Ok(0usize));

        assert_eq!(p_half.try_into() as Result<u8, ()>, Err(()));
        assert_eq!(p_half.try_into() as Result<u16, ()>, Err(()));
        assert_eq!(p_half.try_into() as Result<u32, ()>, Err(()));
        assert_eq!(p_half.try_into() as Result<u64, ()>, Err(()));
        assert_eq!(p_half.try_into() as Result<u128, ()>, Err(()));
        assert_eq!(p_half.try_into() as Result<usize, ()>, Err(()));

        assert_eq!(p_one.try_into(), Ok(1u8));
        assert_eq!(p_one.try_into(), Ok(1u16));
        assert_eq!(p_one.try_into(), Ok(1u32));
        assert_eq!(p_one.try_into(), Ok(1u64));
        assert_eq!(p_one.try_into(), Ok(1u128));
        assert_eq!(p_one.try_into(), Ok(1usize));

        assert_eq!(p_inf.try_into() as Result<u8, ()>, Err(()));
        assert_eq!(p_inf.try_into() as Result<u16, ()>, Err(()));
        assert_eq!(p_inf.try_into() as Result<u32, ()>, Err(()));
        assert_eq!(p_inf.try_into() as Result<u64, ()>, Err(()));
        assert_eq!(p_inf.try_into() as Result<u128, ()>, Err(()));
        assert_eq!(p_inf.try_into() as Result<usize, ()>, Err(()));

        assert_eq!(nan.try_into() as Result<u8, ()>, Err(()));
        assert_eq!(nan.try_into() as Result<u16, ()>, Err(()));
        assert_eq!(nan.try_into() as Result<u32, ()>, Err(()));
        assert_eq!(nan.try_into() as Result<u64, ()>, Err(()));
        assert_eq!(nan.try_into() as Result<u128, ()>, Err(()));
        assert_eq!(nan.try_into() as Result<usize, ()>, Err(()));
    }

    #[test]
    fn fraction_to_f32() {
        let n_inf = Fraction::neg_infinity();
        let n_one = Fraction::one() * -1;
        let n_half = n_one / 2;
        let zero = Fraction::zero();
        let p_one = Fraction::one();
        let p_half = p_one / 2;
        let p_inf = Fraction::infinity();
        let nan = Fraction::nan();

        assert_eq!(n_inf.try_into(), Ok(f32::NEG_INFINITY));
        assert_eq!(n_one.try_into(), Ok(-1f32));
        assert_eq!(n_half.try_into(), Ok(-0.5f32));
        assert_eq!(zero.try_into(), Ok(0f32));
        assert_eq!(p_half.try_into(), Ok(0.5f32));
        assert_eq!(p_one.try_into(), Ok(1f32));
        assert_eq!(p_inf.try_into(), Ok(f32::INFINITY));
        assert!((nan.try_into() as Result<f32, ()>).unwrap_or(0f32).is_nan());
    }

    #[test]
    fn fraction_to_f64() {
        let n_inf = Fraction::neg_infinity();
        let n_one = Fraction::one() * -1;
        let n_half = n_one / 2;
        let zero = Fraction::zero();
        let p_one = Fraction::one();
        let p_half = p_one / 2;
        let p_inf = Fraction::infinity();
        let nan = Fraction::nan();

        assert_eq!(n_inf.try_into(), Ok(f64::NEG_INFINITY));
        assert_eq!(n_one.try_into(), Ok(-1f64));
        assert_eq!(n_half.try_into(), Ok(-0.5f64));
        assert_eq!(zero.try_into(), Ok(0f64));
        assert_eq!(p_half.try_into(), Ok(0.5f64));
        assert_eq!(p_one.try_into(), Ok(1f64));
        assert_eq!(p_inf.try_into(), Ok(f64::INFINITY));
        assert!((nan.try_into() as Result<f64, ()>).unwrap_or(0f64).is_nan());
    }

    #[cfg(feature = "with-bigint")]
    #[test]
    fn fraction_to_bigint() {
        use crate::BigInt;

        let n_inf = Fraction::neg_infinity();
        let n_one = Fraction::one() * -1;
        let n_half = n_one / 2;
        let zero = Fraction::zero();
        let p_one = Fraction::one();
        let p_half = p_one / 2;
        let p_inf = Fraction::infinity();
        let nan = Fraction::nan();

        assert_eq!(n_inf.try_into() as Result<BigInt, ()>, Err(()));
        assert_eq!(n_one.try_into(), Ok(BigInt::one() * -1));
        assert_eq!(n_half.try_into() as Result<BigInt, ()>, Err(()));
        assert_eq!(zero.try_into(), Ok(BigInt::zero()));
        assert_eq!(p_half.try_into() as Result<BigInt, ()>, Err(()));
        assert_eq!(p_one.try_into(), Ok(BigInt::one()));
        assert_eq!(p_inf.try_into() as Result<BigInt, ()>, Err(()));
        assert_eq!(nan.try_into() as Result<BigInt, ()>, Err(()));
    }

    #[cfg(feature = "with-bigint")]
    #[test]
    fn fraction_to_biguint() {
        use crate::BigUint;

        let n_inf = Fraction::neg_infinity();
        let n_one = Fraction::one() * -1;
        let n_half = n_one / 2;
        let zero = Fraction::zero();
        let p_one = Fraction::one();
        let p_half = p_one / 2;
        let p_inf = Fraction::infinity();
        let nan = Fraction::nan();

        assert_eq!(n_inf.try_into() as Result<BigUint, ()>, Err(()));
        assert_eq!(n_one.try_into() as Result<BigUint, ()>, Err(()));
        assert_eq!(n_half.try_into() as Result<BigUint, ()>, Err(()));
        assert_eq!(zero.try_into(), Ok(BigUint::zero()));
        assert_eq!(p_half.try_into() as Result<BigUint, ()>, Err(()));
        assert_eq!(p_one.try_into(), Ok(BigUint::one()));
        assert_eq!(p_inf.try_into() as Result<BigUint, ()>, Err(()));
        assert_eq!(nan.try_into() as Result<BigUint, ()>, Err(()));
    }

    #[cfg(feature = "with-bigint")]
    #[test]
    fn bigfraction_to_i() {
        use crate::BigFraction;

        let n_inf = BigFraction::neg_infinity();
        let n_one = BigFraction::one() * -1;
        let n_half = n_one.clone() / 2;
        let zero = BigFraction::zero();
        let p_one = BigFraction::one();
        let p_half = p_one.clone() / 2;
        let p_inf = BigFraction::infinity();
        let nan = BigFraction::nan();

        assert_eq!(n_inf.clone().try_into() as Result<i8, ()>, Err(()));
        assert_eq!(n_inf.clone().try_into() as Result<i16, ()>, Err(()));
        assert_eq!(n_inf.clone().try_into() as Result<i32, ()>, Err(()));
        assert_eq!(n_inf.clone().try_into() as Result<i64, ()>, Err(()));
        assert_eq!(n_inf.clone().try_into() as Result<i128, ()>, Err(()));
        assert_eq!(n_inf.clone().try_into() as Result<isize, ()>, Err(()));

        assert_eq!(n_one.clone().try_into(), Ok(-1i8));
        assert_eq!(n_one.clone().try_into(), Ok(-1i16));
        assert_eq!(n_one.clone().try_into(), Ok(-1i32));
        assert_eq!(n_one.clone().try_into(), Ok(-1i64));
        assert_eq!(n_one.clone().try_into(), Ok(-1i128));
        assert_eq!(n_one.clone().try_into(), Ok(-1isize));

        assert_eq!(n_half.clone().try_into() as Result<i8, ()>, Err(()));
        assert_eq!(n_half.clone().try_into() as Result<i16, ()>, Err(()));
        assert_eq!(n_half.clone().try_into() as Result<i32, ()>, Err(()));
        assert_eq!(n_half.clone().try_into() as Result<i64, ()>, Err(()));
        assert_eq!(n_half.clone().try_into() as Result<i128, ()>, Err(()));
        assert_eq!(n_half.clone().try_into() as Result<isize, ()>, Err(()));

        assert_eq!(zero.clone().try_into(), Ok(0i8));
        assert_eq!(zero.clone().try_into(), Ok(0i16));
        assert_eq!(zero.clone().try_into(), Ok(0i32));
        assert_eq!(zero.clone().try_into(), Ok(0i64));
        assert_eq!(zero.clone().try_into(), Ok(0i128));
        assert_eq!(zero.clone().try_into(), Ok(0isize));

        assert_eq!(p_half.clone().try_into() as Result<i8, ()>, Err(()));
        assert_eq!(p_half.clone().try_into() as Result<i16, ()>, Err(()));
        assert_eq!(p_half.clone().try_into() as Result<i32, ()>, Err(()));
        assert_eq!(p_half.clone().try_into() as Result<i64, ()>, Err(()));
        assert_eq!(p_half.clone().try_into() as Result<i128, ()>, Err(()));
        assert_eq!(p_half.clone().try_into() as Result<isize, ()>, Err(()));

        assert_eq!(p_one.clone().try_into(), Ok(1i8));
        assert_eq!(p_one.clone().try_into(), Ok(1i16));
        assert_eq!(p_one.clone().try_into(), Ok(1i32));
        assert_eq!(p_one.clone().try_into(), Ok(1i64));
        assert_eq!(p_one.clone().try_into(), Ok(1i128));
        assert_eq!(p_one.clone().try_into(), Ok(1isize));

        assert_eq!(p_inf.clone().try_into() as Result<i8, ()>, Err(()));
        assert_eq!(p_inf.clone().try_into() as Result<i16, ()>, Err(()));
        assert_eq!(p_inf.clone().try_into() as Result<i32, ()>, Err(()));
        assert_eq!(p_inf.clone().try_into() as Result<i64, ()>, Err(()));
        assert_eq!(p_inf.clone().try_into() as Result<i128, ()>, Err(()));
        assert_eq!(p_inf.clone().try_into() as Result<isize, ()>, Err(()));

        assert_eq!(nan.clone().try_into() as Result<i8, ()>, Err(()));
        assert_eq!(nan.clone().try_into() as Result<i16, ()>, Err(()));
        assert_eq!(nan.clone().try_into() as Result<i32, ()>, Err(()));
        assert_eq!(nan.clone().try_into() as Result<i64, ()>, Err(()));
        assert_eq!(nan.clone().try_into() as Result<i128, ()>, Err(()));
        assert_eq!(nan.clone().try_into() as Result<isize, ()>, Err(()));
    }

    #[cfg(feature = "with-bigint")]
    #[test]
    fn bigintfraction_to_i() {
        use crate::GenericFraction;
        use ::numeric::collection::BigInt;

        type BigFraction = GenericFraction<BigInt>;

        let n_inf = BigFraction::neg_infinity();
        let n_one = BigFraction::one() * -1;
        let n_half = n_one.clone() / 2;
        let zero = BigFraction::zero();
        let p_one = BigFraction::one();
        let p_half = p_one.clone() / 2;
        let p_inf = BigFraction::infinity();
        let nan = BigFraction::nan();

        assert_eq!(n_inf.clone().try_into() as Result<i8, ()>, Err(()));
        assert_eq!(n_inf.clone().try_into() as Result<i16, ()>, Err(()));
        assert_eq!(n_inf.clone().try_into() as Result<i32, ()>, Err(()));
        assert_eq!(n_inf.clone().try_into() as Result<i64, ()>, Err(()));
        assert_eq!(n_inf.clone().try_into() as Result<i128, ()>, Err(()));
        assert_eq!(n_inf.clone().try_into() as Result<isize, ()>, Err(()));

        assert_eq!(n_one.clone().try_into(), Ok(-1i8));
        assert_eq!(n_one.clone().try_into(), Ok(-1i16));
        assert_eq!(n_one.clone().try_into(), Ok(-1i32));
        assert_eq!(n_one.clone().try_into(), Ok(-1i64));
        assert_eq!(n_one.clone().try_into(), Ok(-1i128));
        assert_eq!(n_one.clone().try_into(), Ok(-1isize));

        assert_eq!(n_half.clone().try_into() as Result<i8, ()>, Err(()));
        assert_eq!(n_half.clone().try_into() as Result<i16, ()>, Err(()));
        assert_eq!(n_half.clone().try_into() as Result<i32, ()>, Err(()));
        assert_eq!(n_half.clone().try_into() as Result<i64, ()>, Err(()));
        assert_eq!(n_half.clone().try_into() as Result<i128, ()>, Err(()));
        assert_eq!(n_half.clone().try_into() as Result<isize, ()>, Err(()));

        assert_eq!(zero.clone().try_into(), Ok(0i8));
        assert_eq!(zero.clone().try_into(), Ok(0i16));
        assert_eq!(zero.clone().try_into(), Ok(0i32));
        assert_eq!(zero.clone().try_into(), Ok(0i64));
        assert_eq!(zero.clone().try_into(), Ok(0i128));
        assert_eq!(zero.clone().try_into(), Ok(0isize));

        assert_eq!(p_half.clone().try_into() as Result<i8, ()>, Err(()));
        assert_eq!(p_half.clone().try_into() as Result<i16, ()>, Err(()));
        assert_eq!(p_half.clone().try_into() as Result<i32, ()>, Err(()));
        assert_eq!(p_half.clone().try_into() as Result<i64, ()>, Err(()));
        assert_eq!(p_half.clone().try_into() as Result<i128, ()>, Err(()));
        assert_eq!(p_half.clone().try_into() as Result<isize, ()>, Err(()));

        assert_eq!(p_one.clone().try_into(), Ok(1i8));
        assert_eq!(p_one.clone().try_into(), Ok(1i16));
        assert_eq!(p_one.clone().try_into(), Ok(1i32));
        assert_eq!(p_one.clone().try_into(), Ok(1i64));
        assert_eq!(p_one.clone().try_into(), Ok(1i128));
        assert_eq!(p_one.clone().try_into(), Ok(1isize));

        assert_eq!(p_inf.clone().try_into() as Result<i8, ()>, Err(()));
        assert_eq!(p_inf.clone().try_into() as Result<i16, ()>, Err(()));
        assert_eq!(p_inf.clone().try_into() as Result<i32, ()>, Err(()));
        assert_eq!(p_inf.clone().try_into() as Result<i64, ()>, Err(()));
        assert_eq!(p_inf.clone().try_into() as Result<i128, ()>, Err(()));
        assert_eq!(p_inf.clone().try_into() as Result<isize, ()>, Err(()));

        assert_eq!(nan.clone().try_into() as Result<i8, ()>, Err(()));
        assert_eq!(nan.clone().try_into() as Result<i16, ()>, Err(()));
        assert_eq!(nan.clone().try_into() as Result<i32, ()>, Err(()));
        assert_eq!(nan.clone().try_into() as Result<i64, ()>, Err(()));
        assert_eq!(nan.clone().try_into() as Result<i128, ()>, Err(()));
        assert_eq!(nan.clone().try_into() as Result<isize, ()>, Err(()));
    }

    #[cfg(feature = "with-bigint")]
    #[test]
    fn bigfraction_to_u() {
        use crate::BigFraction;

        let n_inf = BigFraction::neg_infinity();
        let n_one = BigFraction::one() * -1;
        let n_half = n_one.clone() / 2;
        let zero = BigFraction::zero();
        let p_one = BigFraction::one();
        let p_half = p_one.clone() / 2;
        let p_inf = BigFraction::infinity();
        let nan = Fraction::nan();

        assert_eq!(n_inf.clone().try_into() as Result<u8, ()>, Err(()));
        assert_eq!(n_inf.clone().try_into() as Result<u16, ()>, Err(()));
        assert_eq!(n_inf.clone().try_into() as Result<u32, ()>, Err(()));
        assert_eq!(n_inf.clone().try_into() as Result<u64, ()>, Err(()));
        assert_eq!(n_inf.clone().try_into() as Result<u128, ()>, Err(()));
        assert_eq!(n_inf.clone().try_into() as Result<usize, ()>, Err(()));

        assert_eq!(n_one.clone().try_into() as Result<u8, ()>, Err(()));
        assert_eq!(n_one.clone().try_into() as Result<u16, ()>, Err(()));
        assert_eq!(n_one.clone().try_into() as Result<u32, ()>, Err(()));
        assert_eq!(n_one.clone().try_into() as Result<u64, ()>, Err(()));
        assert_eq!(n_one.clone().try_into() as Result<u128, ()>, Err(()));
        assert_eq!(n_one.clone().try_into() as Result<usize, ()>, Err(()));

        assert_eq!(n_half.clone().try_into() as Result<u8, ()>, Err(()));
        assert_eq!(n_half.clone().try_into() as Result<u16, ()>, Err(()));
        assert_eq!(n_half.clone().try_into() as Result<u32, ()>, Err(()));
        assert_eq!(n_half.clone().try_into() as Result<u64, ()>, Err(()));
        assert_eq!(n_half.clone().try_into() as Result<u128, ()>, Err(()));
        assert_eq!(n_half.clone().try_into() as Result<usize, ()>, Err(()));

        assert_eq!(zero.clone().try_into(), Ok(0u8));
        assert_eq!(zero.clone().try_into(), Ok(0u16));
        assert_eq!(zero.clone().try_into(), Ok(0u32));
        assert_eq!(zero.clone().try_into(), Ok(0u64));
        assert_eq!(zero.clone().try_into(), Ok(0u128));
        assert_eq!(zero.clone().try_into(), Ok(0usize));

        assert_eq!(p_half.clone().try_into() as Result<u8, ()>, Err(()));
        assert_eq!(p_half.clone().try_into() as Result<u16, ()>, Err(()));
        assert_eq!(p_half.clone().try_into() as Result<u32, ()>, Err(()));
        assert_eq!(p_half.clone().try_into() as Result<u64, ()>, Err(()));
        assert_eq!(p_half.clone().try_into() as Result<u128, ()>, Err(()));
        assert_eq!(p_half.clone().try_into() as Result<usize, ()>, Err(()));

        assert_eq!(p_one.clone().try_into(), Ok(1u8));
        assert_eq!(p_one.clone().try_into(), Ok(1u16));
        assert_eq!(p_one.clone().try_into(), Ok(1u32));
        assert_eq!(p_one.clone().try_into(), Ok(1u64));
        assert_eq!(p_one.clone().try_into(), Ok(1u128));
        assert_eq!(p_one.clone().try_into(), Ok(1usize));

        assert_eq!(p_inf.clone().try_into() as Result<u8, ()>, Err(()));
        assert_eq!(p_inf.clone().try_into() as Result<u16, ()>, Err(()));
        assert_eq!(p_inf.clone().try_into() as Result<u32, ()>, Err(()));
        assert_eq!(p_inf.clone().try_into() as Result<u64, ()>, Err(()));
        assert_eq!(p_inf.clone().try_into() as Result<u128, ()>, Err(()));
        assert_eq!(p_inf.clone().try_into() as Result<usize, ()>, Err(()));

        assert_eq!(nan.clone().try_into() as Result<u8, ()>, Err(()));
        assert_eq!(nan.clone().try_into() as Result<u16, ()>, Err(()));
        assert_eq!(nan.clone().try_into() as Result<u32, ()>, Err(()));
        assert_eq!(nan.clone().try_into() as Result<u64, ()>, Err(()));
        assert_eq!(nan.clone().try_into() as Result<u128, ()>, Err(()));
        assert_eq!(nan.clone().try_into() as Result<usize, ()>, Err(()));
    }

    #[cfg(feature = "with-bigint")]
    #[test]
    fn bigintfraction_to_u() {
        use crate::GenericFraction;
        use ::numeric::collection::BigInt;

        type BigFraction = GenericFraction<BigInt>;

        let n_inf = BigFraction::neg_infinity();
        let n_one = BigFraction::one() * -1;
        let n_half = n_one.clone() / 2;
        let zero = BigFraction::zero();
        let p_one = BigFraction::one();
        let p_half = p_one.clone() / 2;
        let p_inf = BigFraction::infinity();
        let nan = Fraction::nan();

        assert_eq!(n_inf.clone().try_into() as Result<u8, ()>, Err(()));
        assert_eq!(n_inf.clone().try_into() as Result<u16, ()>, Err(()));
        assert_eq!(n_inf.clone().try_into() as Result<u32, ()>, Err(()));
        assert_eq!(n_inf.clone().try_into() as Result<u64, ()>, Err(()));
        assert_eq!(n_inf.clone().try_into() as Result<u128, ()>, Err(()));
        assert_eq!(n_inf.clone().try_into() as Result<usize, ()>, Err(()));

        assert_eq!(n_one.clone().try_into() as Result<u8, ()>, Err(()));
        assert_eq!(n_one.clone().try_into() as Result<u16, ()>, Err(()));
        assert_eq!(n_one.clone().try_into() as Result<u32, ()>, Err(()));
        assert_eq!(n_one.clone().try_into() as Result<u64, ()>, Err(()));
        assert_eq!(n_one.clone().try_into() as Result<u128, ()>, Err(()));
        assert_eq!(n_one.clone().try_into() as Result<usize, ()>, Err(()));

        assert_eq!(n_half.clone().try_into() as Result<u8, ()>, Err(()));
        assert_eq!(n_half.clone().try_into() as Result<u16, ()>, Err(()));
        assert_eq!(n_half.clone().try_into() as Result<u32, ()>, Err(()));
        assert_eq!(n_half.clone().try_into() as Result<u64, ()>, Err(()));
        assert_eq!(n_half.clone().try_into() as Result<u128, ()>, Err(()));
        assert_eq!(n_half.clone().try_into() as Result<usize, ()>, Err(()));

        assert_eq!(zero.clone().try_into(), Ok(0u8));
        assert_eq!(zero.clone().try_into(), Ok(0u16));
        assert_eq!(zero.clone().try_into(), Ok(0u32));
        assert_eq!(zero.clone().try_into(), Ok(0u64));
        assert_eq!(zero.clone().try_into(), Ok(0u128));
        assert_eq!(zero.clone().try_into(), Ok(0usize));

        assert_eq!(p_half.clone().try_into() as Result<u8, ()>, Err(()));
        assert_eq!(p_half.clone().try_into() as Result<u16, ()>, Err(()));
        assert_eq!(p_half.clone().try_into() as Result<u32, ()>, Err(()));
        assert_eq!(p_half.clone().try_into() as Result<u64, ()>, Err(()));
        assert_eq!(p_half.clone().try_into() as Result<u128, ()>, Err(()));
        assert_eq!(p_half.clone().try_into() as Result<usize, ()>, Err(()));

        assert_eq!(p_one.clone().try_into(), Ok(1u8));
        assert_eq!(p_one.clone().try_into(), Ok(1u16));
        assert_eq!(p_one.clone().try_into(), Ok(1u32));
        assert_eq!(p_one.clone().try_into(), Ok(1u64));
        assert_eq!(p_one.clone().try_into(), Ok(1u128));
        assert_eq!(p_one.clone().try_into(), Ok(1usize));

        assert_eq!(p_inf.clone().try_into() as Result<u8, ()>, Err(()));
        assert_eq!(p_inf.clone().try_into() as Result<u16, ()>, Err(()));
        assert_eq!(p_inf.clone().try_into() as Result<u32, ()>, Err(()));
        assert_eq!(p_inf.clone().try_into() as Result<u64, ()>, Err(()));
        assert_eq!(p_inf.clone().try_into() as Result<u128, ()>, Err(()));
        assert_eq!(p_inf.clone().try_into() as Result<usize, ()>, Err(()));

        assert_eq!(nan.clone().try_into() as Result<u8, ()>, Err(()));
        assert_eq!(nan.clone().try_into() as Result<u16, ()>, Err(()));
        assert_eq!(nan.clone().try_into() as Result<u32, ()>, Err(()));
        assert_eq!(nan.clone().try_into() as Result<u64, ()>, Err(()));
        assert_eq!(nan.clone().try_into() as Result<u128, ()>, Err(()));
        assert_eq!(nan.clone().try_into() as Result<usize, ()>, Err(()));
    }

    #[cfg(feature = "with-bigint")]
    #[test]
    fn bigfraction_to_f32() {
        use crate::BigFraction;

        let n_inf = BigFraction::neg_infinity();
        let n_one = BigFraction::one() * -1;
        let n_half = n_one.clone() / 2;
        let zero = BigFraction::zero();
        let p_one = BigFraction::one();
        let p_half = p_one.clone() / 2;
        let p_inf = BigFraction::infinity();
        let nan = BigFraction::nan();

        assert_eq!(n_inf.try_into(), Ok(f32::NEG_INFINITY));
        assert_eq!(n_one.try_into(), Ok(-1f32));
        assert_eq!(n_half.try_into(), Ok(-0.5f32));
        assert_eq!(zero.try_into(), Ok(0f32));
        assert_eq!(p_half.try_into(), Ok(0.5f32));
        assert_eq!(p_one.try_into(), Ok(1f32));
        assert_eq!(p_inf.try_into(), Ok(f32::INFINITY));
        assert!((nan.try_into() as Result<f32, ()>).unwrap_or(0f32).is_nan());
    }

    #[cfg(feature = "with-bigint")]
    #[test]
    fn bigintfraction_to_f32() {
        use crate::GenericFraction;
        use ::numeric::collection::BigInt;

        type BigFraction = GenericFraction<BigInt>;

        let n_inf = BigFraction::neg_infinity();
        let n_one = BigFraction::one() * -1;
        let n_half = n_one.clone() / 2;
        let zero = BigFraction::zero();
        let p_one = BigFraction::one();
        let p_half = p_one.clone() / 2;
        let p_inf = BigFraction::infinity();
        let nan = BigFraction::nan();

        assert_eq!(n_inf.try_into(), Ok(f32::NEG_INFINITY));
        assert_eq!(n_one.try_into(), Ok(-1f32));
        assert_eq!(n_half.try_into(), Ok(-0.5f32));
        assert_eq!(zero.try_into(), Ok(0f32));
        assert_eq!(p_half.try_into(), Ok(0.5f32));
        assert_eq!(p_one.try_into(), Ok(1f32));
        assert_eq!(p_inf.try_into(), Ok(f32::INFINITY));
        assert!((nan.try_into() as Result<f32, ()>).unwrap_or(0f32).is_nan());
    }

    #[cfg(feature = "with-bigint")]
    #[test]
    fn bigfraction_to_f64() {
        use crate::BigFraction;

        let n_inf = BigFraction::neg_infinity();
        let n_one = BigFraction::one() * -1;
        let n_half = n_one.clone() / 2;
        let zero = BigFraction::zero();
        let p_one = BigFraction::one();
        let p_half = p_one.clone() / 2;
        let p_inf = BigFraction::infinity();
        let nan = BigFraction::nan();

        assert_eq!(n_inf.try_into(), Ok(f64::NEG_INFINITY));
        assert_eq!(n_one.try_into(), Ok(-1f64));
        assert_eq!(n_half.try_into(), Ok(-0.5f64));
        assert_eq!(zero.try_into(), Ok(0f64));
        assert_eq!(p_half.try_into(), Ok(0.5f64));
        assert_eq!(p_one.try_into(), Ok(1f64));
        assert_eq!(p_inf.try_into(), Ok(f64::INFINITY));
        assert!((nan.try_into() as Result<f64, ()>).unwrap_or(0f64).is_nan());
    }

    #[cfg(feature = "with-bigint")]
    #[test]
    fn bigintfraction_to_f64() {
        use crate::GenericFraction;
        use ::numeric::collection::BigInt;

        type BigFraction = GenericFraction<BigInt>;

        let n_inf = BigFraction::neg_infinity();
        let n_one = BigFraction::one() * -1;
        let n_half = n_one.clone() / 2;
        let zero = BigFraction::zero();
        let p_one = BigFraction::one();
        let p_half = p_one.clone() / 2;
        let p_inf = BigFraction::infinity();
        let nan = BigFraction::nan();

        assert_eq!(n_inf.try_into(), Ok(f64::NEG_INFINITY));
        assert_eq!(n_one.try_into(), Ok(-1f64));
        assert_eq!(n_half.try_into(), Ok(-0.5f64));
        assert_eq!(zero.try_into(), Ok(0f64));
        assert_eq!(p_half.try_into(), Ok(0.5f64));
        assert_eq!(p_one.try_into(), Ok(1f64));
        assert_eq!(p_inf.try_into(), Ok(f64::INFINITY));
        assert!((nan.try_into() as Result<f64, ()>).unwrap_or(0f64).is_nan());
    }
}
