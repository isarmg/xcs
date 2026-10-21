#[cfg(test)]
mod tests {
    use ::numeric::collection::Integer;
    #[cfg(feature = "with-bigint")]
    use prelude::BigFraction;

    use crate::error::ParseError;
    #[cfg(feature = "with-bigint")]
    use crate::{BigInt, BigUint};

    use crate::{
        Bounded, ConstOne, ConstZero, Fraction, GenericFraction, Num, One, Sign, Signed,
        ToPrimitive, Zero,
    };

    use super::{CheckedAdd, CheckedDiv, CheckedMul, CheckedSub};

    use std::cmp::Ordering;
    use std::collections::HashMap;
    use std::hash::{Hash, Hasher};
    use std::num::FpCategory;
    use std::str::FromStr;

    extern crate rand;
    use self::rand::Rng;
    use generic::GenericInteger;

    type Frac = GenericFraction<u8>;

    fn hash_it(target: &impl Hash) -> u64 {
        use std::collections::hash_map::DefaultHasher;

        let mut h = DefaultHasher::new();
        target.hash(&mut h);
        h.finish()
    }

    generate_ops_tests! (
        NaN => {Frac::nan()};
        NegInf => {Frac::neg_infinity()};
        PosInf => {Frac::infinity()};
        Zero => {Frac::new(0, 1)};
        Half => {Frac::new(1, 2)};
        One => {Frac::new(1, 1)};
        Two => {Frac::new(2, 1)};
        Three => {Frac::new(3, 1)};
        Four => {Frac::new(4, 1)};
    );

    #[test]
    fn op_ord() {
        let pin = Frac::infinity();
        let nin = Frac::neg_infinity();
        let nil = Frac::zero();

        let a = Frac::new(3, 4);
        let b = Frac::new(5, 7);

        assert!(a > b);
        assert!(a > nil);
        assert!(b > nil);
        assert!(nin < nil);
        assert!(nil < pin);
    }

    #[test]
    fn from_i8() {
        let f = Fraction::from(-2i8);
        assert_eq!(Sign::Minus, f.sign().unwrap());
        assert_eq!(2, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());

        let f = Fraction::from(0i8);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(0, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());

        let f = Fraction::from(2i8);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(2, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());
    }

    #[test]
    fn from_u8() {
        let f = Fraction::from(0u8);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(0, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());

        let f = Fraction::from(2u8);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(2, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());

        let f = Fraction::from(u8::MAX);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(u8::MAX as u64, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());
    }

    #[test]
    fn from_i16() {
        let f = Fraction::from(-2i16);
        assert_eq!(Sign::Minus, f.sign().unwrap());
        assert_eq!(2, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());

        let f = Fraction::from(0i16);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(0, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());

        let f = Fraction::from(2i16);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(2, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());
    }

    #[test]
    fn from_u16() {
        let f = Fraction::from(0u16);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(0, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());

        let f = Fraction::from(2u16);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(2, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());
    }

    #[test]
    fn from_i32() {
        let f = Fraction::from(-2i32);
        assert_eq!(Sign::Minus, f.sign().unwrap());
        assert_eq!(2, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());

        let f = Fraction::from(0i32);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(0, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());

        let f = Fraction::from(2i32);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(2, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());
    }

    #[test]
    fn from_u32() {
        let f = Fraction::from(0u32);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(0, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());

        let f = Fraction::from(2u32);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(2, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());
    }

    #[test]
    fn from_i64() {
        let f = Fraction::from(-2i64);
        assert_eq!(Sign::Minus, f.sign().unwrap());
        assert_eq!(2, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());

        let f = Fraction::from(0i64);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(0, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());

        let f = Fraction::from(2i64);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(2, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());
    }

    #[test]
    fn from_u64() {
        let f = Fraction::from(0u64);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(0, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());

        let f = Fraction::from(2u64);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(2, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());
    }

    #[test]
    fn from_i128() {
        let f = Fraction::from(0i128);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(0, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());

        let f = Fraction::from(2i128);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(2, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());

        let f = Fraction::from(-2i128);
        assert_eq!(Sign::Minus, f.sign().unwrap());
        assert_eq!(2, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());

        let f = Fraction::from(22460602606i128);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(22460602606, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());
    }

    #[test]
    fn from_u128() {
        let f = Fraction::from(0u128);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(0, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());

        let f = Fraction::from(2u128);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(2, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());
    }

    #[test]
    fn from_isize() {
        let f = Fraction::from(-2isize);
        assert_eq!(Sign::Minus, f.sign().unwrap());
        assert_eq!(2, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());

        let f = Fraction::from(0isize);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(0, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());

        let f = Fraction::from(2isize);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(2, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());
    }

    #[test]
    fn from_usize() {
        let f = Fraction::from(0usize);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(0, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());

        let f = Fraction::from(2usize);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(2, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());
    }

    #[test]
    fn from_f32() {
        let f = Fraction::from(0f32);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(0, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());

        let f = Fraction::from(0.01f32);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(1, *f.numer().unwrap());
        assert_eq!(100, *f.denom().unwrap());

        let f = Fraction::from(-0.01f32);
        assert_eq!(Sign::Minus, f.sign().unwrap());
        assert_eq!(1, *f.numer().unwrap());
        assert_eq!(100, *f.denom().unwrap());

        let f = Fraction::from(16584253f32);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(16584253u64, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());
    }

    #[test]
    fn from_f64() {
        let f = Fraction::from(0f64);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(0, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());

        let f = Fraction::from(0.01f64);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(1, *f.numer().unwrap());
        assert_eq!(100, *f.denom().unwrap());

        let f = Fraction::from(-0.01f64);
        assert_eq!(Sign::Minus, f.sign().unwrap());
        assert_eq!(1, *f.numer().unwrap());
        assert_eq!(100, *f.denom().unwrap());

        let f = Fraction::from(1658425342060f64);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(1658425342060u64, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());
    }

    #[test]
    #[cfg(feature = "with-bigint")]
    fn from_insanity() {
        let number = "2062065394209534095362056240654064520645230645230645230645230645206452064520645203642530940959212130935957";
        let fraction = format!("{}/1", number);
        let f = BigFraction::from_str_radix(&fraction, 10);
        assert!(f.is_ok());
        let f = f.ok().unwrap();
        assert_eq!(
            BigUint::from_str_radix(&number, 10).ok().unwrap(),
            *f.numer().unwrap()
        );
        assert_eq!(BigUint::from(1u8), *f.denom().unwrap());
    }

    #[test]
    #[cfg(feature = "with-bigint")]
    fn from_bigint() {
        let number = BigInt::from(42);
        let frac = BigFraction::from(number);

        assert_eq!(frac, BigFraction::from((42, 1)));

        let number = BigInt::from(-44);
        let frac = BigFraction::from(number);

        assert_eq!(frac, -BigFraction::from((44, 1)));
    }

    #[test]
    #[cfg(feature = "with-bigint")]
    fn from_biguint() {
        let number = BigUint::from(42u32);
        let frac = BigFraction::from(number);

        assert_eq!(frac, BigFraction::from((42, 1)));
    }

    #[test]
    fn from_extremum() {
        type F8 = GenericFraction<u8>;

        let f = F8::from(u8::MAX);
        assert_eq!(Sign::Plus, f.sign().unwrap());
        assert_eq!(u8::MAX, *f.numer().unwrap());
        assert_eq!(1, *f.denom().unwrap());
    }

    #[test]
    fn hashy() {
        {
            let mut map: HashMap<Fraction, ()> = HashMap::new();

            map.insert(Fraction::from(0.75), ());

            assert!(map.contains_key(&Fraction::new(3u8, 4u8))); // 0.75 == 3/4
            assert!(map.contains_key(&Fraction::new(6u16, 8u16))); // 0.75 == 6/8
            assert!(map.contains_key(&Fraction::new(12u32, 16u32))); // 0.75 == 12/16
            assert!(map.contains_key(&Fraction::new(24u64, 32u64))); // 0.75 == 24/32
            assert!(map.contains_key(&Fraction::new(48u8, 64u16))); // 0.75 == 48/64

            assert!(map.contains_key(&Fraction::from((3i8, 4i8))));
            assert!(map.contains_key(&Fraction::from((6i16, 8i16))));
            assert!(map.contains_key(&Fraction::from((12i32, 16i32))));
            assert!(map.contains_key(&Fraction::from((24i64, 32i64))));
            assert!(map.contains_key(&Fraction::from((48i8, 64i16))));

            assert!(!map.contains_key(&Fraction::from(0.5))); // 0.75 != 1/2
        }

        {
            assert_eq!(hash_it(&Fraction::nan()), hash_it(&Fraction::nan()));
            assert_ne!(hash_it(&Fraction::nan()), hash_it(&Fraction::zero()));
            assert_ne!(
                hash_it(&Fraction::infinity()),
                hash_it(&Fraction::neg_infinity())
            );
            assert_ne!(hash_it(&Fraction::infinity()), hash_it(&Fraction::nan()));
            assert_eq!(
                hash_it(&Fraction::infinity()),
                hash_it(&Fraction::infinity())
            );
            assert_eq!(
                hash_it(&Fraction::neg_infinity()),
                hash_it(&Fraction::neg_infinity())
            );
            assert_eq!(
                hash_it(&Fraction::neg_infinity()),
                hash_it(&Fraction::neg_infinity())
            );

            assert_eq!(
                hash_it(&Fraction::new(1u8, 2u8)),
                hash_it(&Fraction::new(2u8, 4u8))
            );
            assert_eq!(
                hash_it(&Fraction::new(1u8, 0u8)),
                hash_it(&Fraction::new(2u8, 0u8))
            );
            assert_eq!(
                hash_it(&Fraction::new(0u8, 1u8)),
                hash_it(&Fraction::new(0u8, 2u8))
            );

            assert_eq!(hash_it(&Fraction::zero()), hash_it(&Fraction::zero()));
            assert_eq!(hash_it(&Frac::zero()), hash_it(&Frac::neg_zero()));
        }
    }

    #[test]
    fn comparison() {
        assert_eq!(Frac::zero(), Frac::zero());
        assert_eq!(Frac::zero(), Frac::neg_zero());
        assert_eq!(Frac::from(0), Frac::zero());
        assert_eq!(Frac::from(0), Frac::neg_zero());
        assert_eq!(Frac::from(0.5), Frac::new(1u8, 2u8));
        assert_eq!(Frac::from(-0.5), Frac::new_neg(1u8, 2u8));
        assert_ne!(Frac::from(-0.5), Frac::new(1u8, 2u8));

        assert!(!(Frac::zero() < Frac::neg_zero()));
        assert!(!(Frac::neg_zero() < Frac::zero()));

        assert!(!(Frac::zero() > Frac::neg_zero()));
        assert!(!(Frac::neg_zero() > Frac::zero()));

        assert!(Frac::neg_zero() < Frac::new(1u8, 2u8));
        assert!(!(Frac::neg_zero() > Frac::new(1u8, 2u8)));

        assert!(Frac::zero() < Frac::new(1u8, 2u8));
        assert!(!(Frac::zero() > Frac::new(1u8, 2u8)));

        assert!(Frac::new_neg(1u8, 2u8) < Frac::neg_zero());
        assert!(Frac::new_neg(1u8, 2u8) < Frac::zero());

        assert!(!(Frac::new_neg(1u8, 2u8) > Frac::neg_zero()));
        assert!(!(Frac::new_neg(1u8, 2u8) > Frac::zero()));

        assert_eq!(Frac::new(1u8, 2u8), Frac::new(1u8, 2u8));
        assert_eq!(Frac::new_neg(1u8, 2u8), Frac::new_neg(1u8, 2u8));

        assert!(Frac::new_neg(1u8, 2u8) < Frac::new(1u8, 2u8));
        assert!(!(Frac::new(1u8, 2u8) < Frac::new_neg(1u8, 2u8)));
        assert!(!(Frac::new_neg(1u8, 2u8) < Frac::new_neg(1u8, 2u8)));
        assert!(Frac::new_neg(1u8, 2u8) < Frac::new_neg(1u8, 4u8));

        assert!(Frac::new_neg(1u8, 2u8) < Frac::neg_zero());
        assert!(Frac::new_neg(1u8, 2u8) < Frac::zero());
        assert!(!(Frac::neg_zero() < Frac::new_neg(1u8, 2u8)));
        assert!(!(Frac::zero() < Frac::new_neg(1u8, 2u8)));
        assert!(Frac::neg_zero() < Frac::new(1u8, 2u8));
        assert!(Frac::neg_zero() > Frac::new_neg(1u8, 2u8));
        assert!(Frac::zero() > Frac::new_neg(1u8, 2u8));
        assert!(Frac::new(1u8, 2u8) > Frac::neg_zero());
        assert!(!(Frac::new(1u8, 2u8) < Frac::neg_zero()));
        assert!(Frac::zero() < Frac::new(1u8, 2u8));
    }

    #[test]
    fn cmp() {
        /* CMP */
        let nan = Frac::nan();
        let neg_inf = Frac::neg_infinity();
        let neg_one = Frac::one() * -1;
        let zero = Frac::zero();
        let one = Frac::one();
        let inf = Frac::infinity();

        assert_eq!(nan.cmp(&nan), Ordering::Equal);
        assert_eq!(nan.cmp(&neg_inf), Ordering::Less);
        assert_eq!(nan.cmp(&neg_one), Ordering::Less);
        assert_eq!(nan.cmp(&zero), Ordering::Less);
        assert_eq!(nan.cmp(&one), Ordering::Less);
        assert_eq!(nan.cmp(&inf), Ordering::Less);

        assert_eq!(neg_inf.cmp(&nan), Ordering::Greater);
        assert_eq!(neg_inf.cmp(&neg_inf), Ordering::Equal);
        assert_eq!(neg_inf.cmp(&neg_one), Ordering::Less);
        assert_eq!(neg_inf.cmp(&zero), Ordering::Less);
        assert_eq!(neg_inf.cmp(&one), Ordering::Less);
        assert_eq!(neg_inf.cmp(&inf), Ordering::Less);

        assert_eq!(neg_one.cmp(&nan), Ordering::Greater);
        assert_eq!(neg_one.cmp(&neg_inf), Ordering::Greater);
        assert_eq!(neg_one.cmp(&neg_one), Ordering::Equal);
        assert_eq!(neg_one.cmp(&zero), Ordering::Less);
        assert_eq!(neg_one.cmp(&one), Ordering::Less);
        assert_eq!(neg_one.cmp(&inf), Ordering::Less);

        assert_eq!(zero.cmp(&nan), Ordering::Greater);
        assert_eq!(zero.cmp(&neg_inf), Ordering::Greater);
        assert_eq!(zero.cmp(&neg_one), Ordering::Greater);
        assert_eq!(zero.cmp(&zero), Ordering::Equal);
        assert_eq!(zero.cmp(&one), Ordering::Less);
        assert_eq!(zero.cmp(&inf), Ordering::Less);

        assert_eq!(one.cmp(&nan), Ordering::Greater);
        assert_eq!(one.cmp(&neg_inf), Ordering::Greater);
        assert_eq!(one.cmp(&neg_one), Ordering::Greater);
        assert_eq!(one.cmp(&zero), Ordering::Greater);
        assert_eq!(one.cmp(&one), Ordering::Equal);
        assert_eq!(one.cmp(&inf), Ordering::Less);

        assert_eq!(inf.cmp(&nan), Ordering::Greater);
        assert_eq!(inf.cmp(&neg_inf), Ordering::Greater);
        assert_eq!(inf.cmp(&neg_one), Ordering::Greater);
        assert_eq!(inf.cmp(&zero), Ordering::Greater);
        assert_eq!(inf.cmp(&one), Ordering::Greater);
        assert_eq!(inf.cmp(&inf), Ordering::Equal);
    }

    #[test]
    fn floor() {
        assert_eq!(Frac::zero(), Frac::new(1, 3).floor());
        assert_eq!(Frac::one() * -1, Frac::new_neg(1, 3).floor());
    }

    #[test]
    fn ceil() {
        assert_eq!(Frac::zero(), Frac::new_neg(1, 3).ceil());
        assert_eq!(Frac::one(), Frac::new(1, 3).ceil());
    }

    #[test]
    fn from_str() {
        assert_eq!(Ok(Frac::zero()), Frac::from_str("0"));
        assert_eq!(Ok(Frac::zero()), Frac::from_str("-0"));
        assert_eq!(Ok(Frac::zero()), Frac::from_str("+0"));

        assert_eq!(Ok(Frac::zero()), Frac::from_str("0.0"));
        assert_eq!(Ok(Frac::zero()), Frac::from_str("-0.0"));
        assert_eq!(Ok(Frac::zero()), Frac::from_str("+0.0"));

        assert_eq!(Ok(Fraction::zero()), Fraction::from_str("0.000000"));
        assert_eq!(Ok(Fraction::zero()), Fraction::from_str("-0.000000"));
        assert_eq!(Ok(Fraction::zero()), Fraction::from_str("+0.000000"));

        assert_eq!(Ok(Fraction::zero()), Fraction::from_str("0/1"));
        assert_eq!(Ok(Fraction::zero()), Fraction::from_str("-0/1"));
        assert_eq!(Ok(Fraction::zero()), Fraction::from_str("+0/1"));

        assert_eq!(
            Ok(Fraction::from(10000000000000_u64)),
            Fraction::from_str("10000000000000.0000000000")
        );

        #[cfg(feature = "with-bigint")]
        {
            assert_eq!(
                Ok(BigFraction::zero()),
                BigFraction::from_str(
                    "00000000000000000000000000.0000000000000000000000000000000000000000000"
                )
            );
            assert_eq!(
                Ok(BigFraction::zero()),
                BigFraction::from_str(
                    "-0000000000000000000000000.0000000000000000000000000000000000000000000"
                )
            );
            assert_eq!(
                Ok(BigFraction::zero()),
                BigFraction::from_str(
                    "+0000000000000000000000000.0000000000000000000000000000000000000000000"
                )
            );

            assert_eq!(
                Ok(BigFraction::zero()),
                BigFraction::from_str("00000000000000000000000000/1")
            );
            assert_eq!(
                Ok(BigFraction::zero()),
                BigFraction::from_str("-0000000000000000000000000/1")
            );
            assert_eq!(
                Ok(BigFraction::zero()),
                BigFraction::from_str("+0000000000000000000000000/1")
            );
        }

        assert_eq!(Ok(Frac::one()), Frac::from_str("1"));
        assert_eq!(Ok(Frac::new_neg(1, 1)), Frac::from_str("-1"));
        assert_eq!(Ok(Frac::one()), Frac::from_str("+1"));

        assert_eq!(Ok(Frac::one()), Frac::from_str("1.0"));
        assert_eq!(Ok(Frac::new_neg(1, 1)), Frac::from_str("-1.0"));
        assert_eq!(Ok(Frac::one()), Frac::from_str("+1.00"));

        assert_eq!(Ok(Frac::one()), Frac::from_str("1/1"));
        assert_eq!(Ok(Frac::new_neg(1, 1)), Frac::from_str("-1/1"));
        assert_eq!(Ok(Frac::one()), Frac::from_str("+1/1"));

        assert_eq!(Ok(Frac::new(1, 2)), Frac::from_str("0.5"));
        assert_eq!(Ok(Frac::new(1, 2)), Frac::from_str("1/2"));

        assert_eq!(
            Ok(Fraction::new(3333u64, 5000u64)),
            Fraction::from_str("0.6666")
        );
        assert_eq!(
            Ok(Fraction::new(3333u64, 5000u64)),
            Fraction::from_str("3333/5000")
        );

        assert_eq!(Err(ParseError::ParseIntError), Frac::from_str("test"));
        assert_eq!(Err(ParseError::ParseIntError), Frac::from_str("1test"));
        assert_eq!(Err(ParseError::ParseIntError), Frac::from_str("1.26t8"));

        // this is due to the std library which issues ParseIntError on the whole part overflow
        assert_eq!(Err(ParseError::ParseIntError), Frac::from_str("120202040"));
        assert_eq!(Err(ParseError::ParseIntError), Frac::from_str("1.20602604"));

        assert_eq!(Err(ParseError::OverflowError), Frac::from_str("255.255"));

        assert_eq!(Err(ParseError::ParseIntError), Frac::from_str("256/256"));
    }

    #[test]
    fn from_str_unsigned_rejects_component_signs() {
        for input in ["1/+2", "1.+5", "-+1"] {
            assert_eq!(
                Err(ParseError::ParseIntError),
                Frac::from_str(input),
                "input should not place a sign inside a component: {input}"
            );
        }
    }

    #[test]
    fn from_str_separator_contract() {
        for (input, expected) in [
            ("1", Ok(Frac::one())),
            ("1/2", Ok(Frac::new(1, 2))),
            ("1.25", Ok(Frac::new(125, 100))),
            ("1.", Ok(Frac::one())),
        ] {
            assert_eq!(
                Frac::from_str(input),
                expected,
                "input should parse: {input}"
            );
        }

        for input in ["1/2.5", "1.2/3", "1..2", "1//2", "/2", "1/", ".5"] {
            assert_eq!(
                Err(ParseError::ParseIntError),
                Frac::from_str(input),
                "input should be rejected: {input}"
            );
        }
    }

    #[test]
    fn from_str_zero_denominator() {
        assert_eq!(Ok(Fraction::infinity()), Fraction::from_str("1/0"));
        assert_eq!(Ok(Fraction::neg_infinity()), Fraction::from_str("-1/0"));
        assert_eq!(Ok(Fraction::infinity()), Fraction::from_str("+1/0"));
        assert_eq!(Ok(Fraction::nan()), Fraction::from_str("0/0"));
        assert_eq!(Ok(Fraction::infinity()), Fraction::from_str("1/000"));

        assert_eq!(
            Ok(GenericFraction::<u64>::infinity()),
            GenericFraction::<u64>::from_str("1/0")
        );
        assert_eq!(
            Ok(GenericFraction::<u64>::nan()),
            GenericFraction::<u64>::from_str("0/0")
        );

        assert_eq!(
            Ok(GenericFraction::<u8>::infinity()),
            GenericFraction::<u8>::from_str("1/0")
        );
        assert_eq!(
            Ok(GenericFraction::<u8>::neg_infinity()),
            GenericFraction::<u8>::from_str("-1/0")
        );
        assert_eq!(
            Ok(GenericFraction::<u8>::infinity()),
            GenericFraction::<u8>::from_str("+1/0")
        );
        assert_eq!(
            Ok(GenericFraction::<u8>::infinity()),
            GenericFraction::<u8>::from_str("1/000")
        );
        assert_eq!(
            Ok(GenericFraction::<u8>::new(1u8, 2u8)),
            GenericFraction::<u8>::from_str("2/4")
        );
        assert_eq!(
            Ok(GenericFraction::<u8>::nan()),
            GenericFraction::<u8>::from_str("0/0")
        );
    }

    #[test]
    fn from_str_component_sign_rejected_for_signed_storage() {
        type SignedFrac = GenericFraction<i8>;

        for (input, sign, numer, denom) in [
            ("1/2", Sign::Plus, 1i8, 2i8),
            ("+1/2", Sign::Plus, 1i8, 2i8),
            ("-1/2", Sign::Minus, 1i8, 2i8),
            ("-127", Sign::Minus, 127i8, 1i8),
        ] {
            match SignedFrac::from_str(input).unwrap() {
                SignedFrac::Rational(actual_sign, ratio) => {
                    assert_eq!(sign, actual_sign);
                    assert_eq!(&numer, ratio.numer());
                    assert_eq!(&denom, ratio.denom());
                    assert!(*ratio.numer() >= 0);
                    assert!(*ratio.denom() >= 0);
                }
                other => panic!("expected rational, got {:?}", other),
            }
        }

        for input in [
            "1/-2", "1/+2", "--1/2", "+-1/2", "+-1", "1.-2", "1/2.3", "1.2/3", "1/-0", "1/+0",
            "-128", "-128/1", "1/-128", "-1/-128",
        ] {
            assert_eq!(
                Err(ParseError::ParseIntError),
                SignedFrac::from_str(input),
                "input should not place a sign inside a component: {input}"
            );
        }
    }

    #[test]
    fn new_generic() {
        {
            type F = GenericFraction<u8>;

            let f12 = F::new_generic(Sign::Plus, 1i8, 2u8).unwrap();
            let f34 = F::new_generic(Sign::Minus, 3i16, 4u32).unwrap();
            let f56 = F::new_generic(Sign::Plus, -5i64, 6u128).unwrap();
            let f78 = F::new_generic(Sign::Minus, 7usize, -8isize).unwrap();

            #[cfg(feature = "with-bigint")]
            {
                let fbig =
                    F::new_generic(Sign::Minus, -BigInt::from(254), BigUint::from(255u32)).unwrap();

                assert_eq!(
                    (
                        fbig.sign().unwrap(),
                        *fbig.numer().unwrap(),
                        *fbig.denom().unwrap()
                    ),
                    (Sign::Plus, 254u8, 255u8)
                );
            }

            assert_eq!(
                (
                    f12.sign().unwrap(),
                    *f12.numer().unwrap(),
                    *f12.denom().unwrap()
                ),
                (Sign::Plus, 1u8, 2u8)
            );
            assert_eq!(
                (
                    f34.sign().unwrap(),
                    *f34.numer().unwrap(),
                    *f34.denom().unwrap()
                ),
                (Sign::Minus, 3u8, 4u8)
            );
            assert_eq!(
                (
                    f56.sign().unwrap(),
                    *f56.numer().unwrap(),
                    *f56.denom().unwrap()
                ),
                (Sign::Minus, 5u8, 6u8)
            );
            assert_eq!(
                (
                    f78.sign().unwrap(),
                    *f78.numer().unwrap(),
                    *f78.denom().unwrap()
                ),
                (Sign::Plus, 7u8, 8u8)
            );

            assert_eq!(None, F::new_generic(Sign::Plus, 256, 1)); // overflow
            assert_eq!(None, F::new_generic(Sign::Plus, 1, 257)); // overflow
        }

        {
            type F = GenericFraction<i8>;

            let f12 = F::new_generic(Sign::Plus, 1i8, 2u8).unwrap();
            let f34 = F::new_generic(Sign::Minus, 3i16, 4u32).unwrap();
            let f56 = F::new_generic(Sign::Plus, -5i64, 6u128).unwrap();
            let f78 = F::new_generic(Sign::Minus, 7usize, -8isize).unwrap();

            assert_eq!(
                (
                    f12.sign().unwrap(),
                    *f12.numer().unwrap(),
                    *f12.denom().unwrap()
                ),
                (Sign::Plus, 1i8, 2i8)
            );
            assert_eq!(
                (
                    f34.sign().unwrap(),
                    *f34.numer().unwrap(),
                    *f34.denom().unwrap()
                ),
                (Sign::Minus, 3i8, 4i8)
            );
            assert_eq!(
                (
                    f56.sign().unwrap(),
                    *f56.numer().unwrap(),
                    *f56.denom().unwrap()
                ),
                (Sign::Minus, 5i8, 6i8)
            );
            assert_eq!(
                (
                    f78.sign().unwrap(),
                    *f78.numer().unwrap(),
                    *f78.denom().unwrap()
                ),
                (Sign::Plus, 7i8, 8i8)
            );

            assert_eq!(None, F::new_generic(Sign::Plus, 128, 1)); // overflow
            assert_eq!(None, F::new_generic(Sign::Plus, 256, 1)); // overflow

            #[cfg(feature = "with-bigint")]
            {
                let fbig =
                    F::new_generic(Sign::Minus, -BigInt::from(126), BigUint::from(127u8)).unwrap();

                assert_eq!(
                    (
                        fbig.sign().unwrap(),
                        *fbig.numer().unwrap(),
                        *fbig.denom().unwrap()
                    ),
                    (Sign::Plus, 126i8, 127i8)
                );
            }
        }

        #[cfg(feature = "with-bigint")]
        {
            type F = GenericFraction<BigUint>;

            let f12 = F::new_generic(Sign::Plus, 1i8, 2u8).unwrap();
            let f34 = F::new_generic(Sign::Minus, 3i16, 4u32).unwrap();
            let f56 = F::new_generic(Sign::Plus, -5i64, 6u128).unwrap();
            let f78 = F::new_generic(Sign::Minus, 7usize, -8isize).unwrap();
            let fbig =
                F::new_generic(Sign::Minus, -BigInt::from(254), BigUint::from(255u32)).unwrap();

            assert_eq!(
                (
                    f12.sign().unwrap(),
                    f12.numer().unwrap(),
                    f12.denom().unwrap()
                ),
                (Sign::Plus, &BigUint::from(1u8), &BigUint::from(2u8))
            );
            assert_eq!(
                (
                    f34.sign().unwrap(),
                    f34.numer().unwrap(),
                    f34.denom().unwrap()
                ),
                (Sign::Minus, &BigUint::from(3u8), &BigUint::from(4u8))
            );
            assert_eq!(
                (
                    f56.sign().unwrap(),
                    f56.numer().unwrap(),
                    f56.denom().unwrap()
                ),
                (Sign::Minus, &BigUint::from(5u8), &BigUint::from(6u8))
            );
            assert_eq!(
                (
                    f78.sign().unwrap(),
                    f78.numer().unwrap(),
                    f78.denom().unwrap()
                ),
                (Sign::Plus, &BigUint::from(7u8), &BigUint::from(8u8))
            );
            assert_eq!(
                (
                    fbig.sign().unwrap(),
                    fbig.numer().unwrap(),
                    fbig.denom().unwrap()
                ),
                (Sign::Plus, &BigUint::from(254u8), &BigUint::from(255u8))
            );
        }

        #[cfg(feature = "with-bigint")]
        {
            type F = GenericFraction<BigInt>;

            let f12 = F::new_generic(Sign::Plus, 1i8, 2u8).unwrap();
            let f34 = F::new_generic(Sign::Minus, 3i16, 4u32).unwrap();
            let f56 = F::new_generic(Sign::Plus, -5i64, 6u128).unwrap();
            let f78 = F::new_generic(Sign::Minus, 7usize, -8isize).unwrap();
            let fbig =
                F::new_generic(Sign::Minus, -BigInt::from(254), BigUint::from(255u32)).unwrap();

            assert_eq!(
                (
                    f12.sign().unwrap(),
                    f12.numer().unwrap(),
                    f12.denom().unwrap()
                ),
                (Sign::Plus, &BigInt::from(1u8), &BigInt::from(2u8))
            );
            assert_eq!(
                (
                    f34.sign().unwrap(),
                    f34.numer().unwrap(),
                    f34.denom().unwrap()
                ),
                (Sign::Minus, &BigInt::from(3u8), &BigInt::from(4u8))
            );
            assert_eq!(
                (
                    f56.sign().unwrap(),
                    f56.numer().unwrap(),
                    f56.denom().unwrap()
                ),
                (Sign::Minus, &BigInt::from(5u8), &BigInt::from(6u8))
            );
            assert_eq!(
                (
                    f78.sign().unwrap(),
                    f78.numer().unwrap(),
                    f78.denom().unwrap()
                ),
                (Sign::Plus, &BigInt::from(7u8), &BigInt::from(8u8))
            );
            assert_eq!(
                (
                    fbig.sign().unwrap(),
                    fbig.numer().unwrap(),
                    fbig.denom().unwrap()
                ),
                (Sign::Plus, &BigInt::from(254u8), &BigInt::from(255u8))
            );
        }

        {
            type F = GenericFraction<u128>;

            let f1 = F::new_generic(Sign::Plus, 123456788u64, 123456789i32).unwrap();
            let f2 = F::new_generic(Sign::Minus, 1234567890122u64, -1234567890123i64).unwrap();

            assert_eq!(
                (
                    f1.sign().unwrap(),
                    *f1.numer().unwrap(),
                    *f1.denom().unwrap()
                ),
                (Sign::Plus, 123456788u128, 123456789u128)
            );
            assert_eq!(
                (
                    f2.sign().unwrap(),
                    *f2.numer().unwrap(),
                    *f2.denom().unwrap()
                ),
                (Sign::Plus, 1234567890122u128, 1234567890123u128)
            );
        }
    }

    #[test]
    fn sign() {
        let p = Sign::Plus;
        let m = Sign::Minus;

        assert_ne!(p, m);
        assert_eq!(p, Sign::Plus);
        assert_eq!(m, Sign::Minus);

        assert_eq!(m, p * m);
        assert_eq!(p, p * p);
        assert_eq!(p, m * m);
        assert_eq!(m, m * p);

        assert!(p.is_positive());
        assert!(!p.is_negative());

        assert!(!m.is_positive());
        assert!(m.is_negative());
    }

    #[test]
    fn fraction() {
        assert_eq!(Frac::nan(), Frac::new(0, 0));
        assert_eq!(Frac::infinity(), Frac::new(1, 0));

        assert!(Frac::nan().numer().is_none());
        assert!(Frac::nan().denom().is_none());

        assert_eq!(
            Fraction::new(5u64, 8u64),
            Fraction::from_fraction(Frac::new(5u8, 8u8))
        );
        assert_eq!(Fraction::nan(), Fraction::from_fraction(Frac::nan()));
        assert_eq!(
            Fraction::infinity(),
            Fraction::from_fraction(Frac::infinity())
        );

        assert_eq!(Frac::min_value(), Frac::new_neg(255, 1));
        assert_eq!(Frac::max_value(), Frac::new(255, 1));

        assert_ne!(Frac::neg_infinity(), Frac::infinity());
        assert_ne!(Frac::nan(), Frac::zero());
        assert_ne!(Frac::zero(), Frac::nan());
        assert_ne!(Frac::new_neg(1, 2), Frac::new(1, 2));
        assert_eq!(Frac::neg_zero(), Frac::zero());
        assert_ne!(Frac::infinity(), Frac::nan());

        assert!(!(Frac::infinity() > Frac::infinity()));
        assert!((Frac::infinity() > Frac::neg_infinity()));
        assert!(!(Frac::neg_infinity() > Frac::infinity()));

        assert!(Frac::infinity() > Frac::max_value());
        assert!(Frac::min_value() > Frac::neg_infinity());

        assert_eq!(-Frac::infinity(), Frac::neg_infinity());

        assert_eq!(-&Frac::nan(), Frac::nan());
        assert_eq!(-&Frac::infinity(), Frac::neg_infinity());
        assert_eq!(-&Frac::one(), Frac::new_neg(1, 1));

        assert_eq!(-&Frac::zero(), Frac::zero());

        assert_eq!(
            Frac::new_neg(1, 1) + Frac::new_neg(1, 1),
            Frac::new_neg(2, 1)
        );
        assert_eq!(
            &Frac::new_neg(1, 1) + &Frac::new_neg(1, 1),
            Frac::new_neg(2, 1)
        );

        assert_eq!(Frac::new_neg(1, 1) - Frac::new_neg(1, 1), Frac::zero());
        assert_eq!(Frac::new_neg(1, 1) - Frac::new_neg(2, 1), Frac::one());
        assert_eq!(&Frac::new_neg(1, 1) - &Frac::new_neg(1, 1), Frac::zero());
        assert_eq!(&Frac::new_neg(1, 1) - &Frac::new_neg(2, 1), Frac::one());

        assert_eq!(Frac::new(1, 255), Frac::min_positive_value());

        assert!(Frac::infinity().is_infinite());
        assert!(Frac::neg_infinity().is_infinite());
        assert!(!Frac::one().is_infinite());

        assert!(!Frac::infinity().is_finite());
        assert!(!Frac::neg_infinity().is_finite());
        assert!(Frac::one().is_finite());

        assert_eq!(FpCategory::Normal, Frac::one().classify());
        assert_eq!(FpCategory::Infinite, Frac::infinity().classify());
        assert_eq!(FpCategory::Zero, Frac::zero().classify());
        assert_eq!(FpCategory::Nan, Frac::nan().classify());

        assert_eq!(Frac::nan(), Frac::nan().floor());
        assert_eq!(Frac::one(), Frac::new(3, 2).floor());

        assert_eq!(Frac::nan(), Frac::nan().ceil());
        assert_eq!(Frac::one(), Frac::new(1, 2).ceil());

        assert_eq!(Frac::nan(), Frac::nan().round());
        assert_eq!(Frac::one(), Frac::new(1, 2).round());
        assert_eq!(Frac::new(2, 1), Frac::new(3, 2).round());
        assert_eq!(Frac::one(), Frac::new(4, 3).round());

        assert_eq!(Frac::nan(), Frac::nan().trunc());
        assert_eq!(Frac::zero(), Frac::new(1, 2).trunc());
        assert_eq!(Frac::one(), Frac::new(3, 2).trunc());

        assert_eq!(Frac::nan(), Frac::nan().fract());
        assert_eq!(Frac::new(1, 2), Frac::new(1, 2).fract());
        assert_eq!(Frac::new(1, 2), Frac::new(3, 2).fract());

        assert!(!Frac::nan().is_sign_positive());
        assert!(!Frac::nan().is_sign_negative());

        assert!(Frac::infinity().is_sign_positive());
        assert!(!Frac::neg_infinity().is_sign_positive());

        assert!(!Frac::infinity().is_sign_negative());
        assert!(Frac::neg_infinity().is_sign_negative());

        assert!(Frac::one().is_sign_positive());
        assert!(!Frac::one().is_sign_negative());

        assert!(!Frac::new_neg(1, 1).is_sign_positive());
        assert!(Frac::new_neg(1, 1).is_sign_negative());

        assert_eq!(
            Frac::new(3, 1),
            Frac::one().mul_add(Frac::new(2, 1), Frac::one())
        );

        assert_eq!(Frac::nan(), Frac::nan().recip());
        assert_eq!(Frac::zero(), Frac::infinity().recip());
        assert_eq!(Frac::zero(), Frac::neg_infinity().recip());
        assert_eq!(Frac::infinity(), Frac::zero().recip());
        assert_eq!(Frac::new(2, 1), Frac::new(1, 2).recip());
    }

    #[test]
    fn from_str_radix() {
        assert_eq!(Frac::one(), Frac::from_str_radix("5/5", 10).unwrap());
        assert_eq!(Frac::one(), Frac::from_str_radix("+5/5", 10).unwrap());
        assert_eq!(Frac::new(1, 2), Frac::from_str_radix("+5/10", 10).unwrap());
        assert_eq!(
            Frac::new_neg(4, 3),
            Frac::from_str_radix("-4/3", 10).unwrap()
        );
    }

    #[test]
    fn from_str_radix_rejects_invalid_bases() {
        for radix in [0, 1, 37] {
            assert_eq!(
                Err(ParseError::UnsupportedBase),
                Frac::from_str_radix("1/2", radix)
            );
        }
        assert_eq!(Ok(Frac::one()), Frac::from_str_radix("10/10", 2));
        assert_eq!(Ok(Frac::one()), Frac::from_str_radix("z/z", 36));
    }

    #[test]
    fn from_str_radix_unsigned_rejects_component_sign() {
        assert_eq!(
            Err(ParseError::ParseIntError),
            Frac::from_str_radix("1/+2", 10)
        );
    }

    #[test]
    fn from_str_radix_component_sign_rejected() {
        type SignedFrac = GenericFraction<i8>;

        for (input, radix, sign, numer, denom) in [
            ("1/2", 10, Sign::Plus, 1i8, 2i8),
            ("+1/2", 10, Sign::Plus, 1i8, 2i8),
            ("-1/2", 10, Sign::Minus, 1i8, 2i8),
            ("b/10", 16, Sign::Plus, 11i8, 16i8),
            ("-b/10", 16, Sign::Minus, 11i8, 16i8),
        ] {
            match SignedFrac::from_str_radix(input, radix).unwrap() {
                SignedFrac::Rational(actual_sign, ratio) => {
                    assert_eq!(sign, actual_sign);
                    assert_eq!(&numer, ratio.numer());
                    assert_eq!(&denom, ratio.denom());
                    assert!(*ratio.numer() >= 0);
                    assert!(*ratio.denom() >= 0);
                }
                other => panic!("expected rational, got {:?}", other),
            }
        }

        for input in [
            "1/-2", "1/+2", "--1/2", "+-1/2", "1/-0", "1/+0", "-128/1", "1/-128", "1/2/3", "1",
        ] {
            assert_eq!(
                Err(ParseError::ParseIntError),
                SignedFrac::from_str_radix(input, 10),
                "input should not place a sign inside a radix component: {input}"
            );
        }
        assert_eq!(
            Err(ParseError::ZeroDenominator),
            SignedFrac::from_str_radix("1/0", 10)
        );
        assert_eq!(
            Err(ParseError::ZeroDenominator),
            SignedFrac::from_str_radix("b/0", 16)
        );
        assert_eq!("Zero denominator", ParseError::ZeroDenominator.to_string());
    }

    #[test]
    fn signed() {
        // abs
        assert_eq!(Frac::one(), <Frac as Signed>::abs(&Frac::new_neg(1, 1)));
        assert_eq!(Frac::nan(), <Frac as Signed>::abs(&Frac::nan()));
        assert_eq!(Frac::infinity(), <Frac as Signed>::abs(&Frac::infinity()));
        assert_eq!(
            Frac::infinity(),
            <Frac as Signed>::abs(&Frac::neg_infinity())
        );

        // abs_sub
        assert_eq!(Frac::nan(), Frac::nan().abs_sub(&Frac::nan()));
        assert_eq!(Frac::nan(), Frac::infinity().abs_sub(&Frac::nan()));
        assert_eq!(Frac::zero(), Frac::infinity().abs_sub(&Frac::infinity()));
        assert_eq!(
            Frac::infinity(),
            Frac::infinity().abs_sub(&Frac::neg_infinity())
        );
        assert_eq!(
            Frac::zero(),
            Frac::neg_infinity().abs_sub(&Frac::neg_infinity())
        );
        assert_eq!(
            Frac::zero(),
            Frac::neg_infinity().abs_sub(&Frac::infinity())
        );
        assert_eq!(Frac::infinity(), Frac::infinity().abs_sub(&Frac::one()));
        assert_eq!(
            Frac::infinity(),
            Frac::infinity().abs_sub(&Frac::new_neg(1, 1))
        );
        assert_eq!(Frac::zero(), Frac::neg_infinity().abs_sub(&Frac::one()));
        assert_eq!(Frac::nan(), Frac::one().abs_sub(&Frac::nan()));
        assert_eq!(Frac::zero(), Frac::one().abs_sub(&Frac::infinity()));
        assert_eq!(Frac::infinity(), Frac::one().abs_sub(&Frac::neg_infinity()));
        assert_eq!(
            Frac::neg_infinity(),
            Frac::new_neg(1, 1).abs_sub(&Frac::neg_infinity())
        );
        assert_eq!(Frac::one(), Frac::new(2, 1).abs_sub(&Frac::one()));
        assert_eq!(Frac::one(), Frac::one().abs_sub(&Frac::new(2, 1)));

        // signum
        assert_eq!(Frac::nan(), Frac::nan().signum());
        assert_eq!(Frac::one(), Frac::infinity().signum());
        assert_eq!(Frac::one(), Frac::zero().signum());
        assert_eq!(Frac::one(), Frac::one().signum());
        assert_eq!(-Frac::one(), Frac::neg_infinity().signum());
        assert_eq!(-Frac::one(), Frac::new_neg(1, 1).signum());
    }

    #[test]
    fn to_primitive() {
        assert!(Frac::nan().to_i64().is_none());
        assert!(Frac::nan().to_u64().is_none());

        assert!(Frac::infinity().to_i64().is_none());
        assert!(Frac::infinity().to_u64().is_none());

        assert!(Frac::neg_infinity().to_i64().is_none());
        assert!(Frac::neg_infinity().to_u64().is_none());

        assert_eq!(Some(1), Frac::one().to_i64());
        assert_eq!(Some(1), Frac::one().to_u64());

        assert!(Frac::new(1, 2).to_i64().is_none());
        assert!(Frac::new(1, 2).to_u64().is_none());

        assert_eq!(Some(-1), Frac::new_neg(1, 1).to_i64());
        assert!(Frac::new_neg(1, 1).to_u64().is_none());

        /* f64 */

        assert!(Frac::nan().to_f64().unwrap().is_nan());
        assert_eq!(f64::INFINITY, Frac::infinity().to_f64().unwrap());
        assert_eq!(
            f64::NEG_INFINITY,
            Frac::neg_infinity().to_f64().unwrap()
        );

        assert_eq!(1f64, Frac::one().to_f64().unwrap());
    }

    #[test]
    fn summing_iterator() {
        let values = vec![Fraction::new(2u64, 3u64), Fraction::new(1u64, 3u64)];
        let sum: Fraction = values.iter().sum();
        assert_eq!(sum, Fraction::new(1u8, 1u8));
    }

    #[test]
    fn product_iterator() {
        let values = vec![Fraction::new(2u64, 3u64), Fraction::new(1u64, 3u64)];
        let product: Fraction = values.iter().product();
        assert_eq!(product, Fraction::new(2u8, 9u8));
    }

    #[test]
    fn fraction_from_float() {
        macro_rules! test_for_smaller_t {
            ( $($t:ty),*) => {
                $(
                    // f32 tests
                    let f = GenericFraction::<$t>::from(-f32::NAN);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(f32::NAN);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(-f32::MIN);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(f32::MIN);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(-f32::MAX);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(f32::MAX);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(-f32::INFINITY);
                    assert_eq!(format!("{}", f), "-inf");
                    let f = GenericFraction::<$t>::from(f32::INFINITY);
                    assert_eq!(format!("{}", f), "inf");
                    let f = GenericFraction::<$t>::from(-0.0_f32);
                    assert_eq!(format!("{}", f), "0");
                    let f = GenericFraction::<$t>::from(0.0_f32);
                    assert_eq!(format!("{}", f), "0");
                    let f = GenericFraction::<$t>::from(-1.0_f32);
                    assert_eq!(format!("{}", f), "-1");
                    let f = GenericFraction::<$t>::from(1.0_f32);
                    assert_eq!(format!("{}", f), "1");
                    // f64 tests
                    let f = GenericFraction::<$t>::from(-f64::NAN);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(f64::NAN);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(-f64::MIN);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(f64::MIN);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(-f64::MAX);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(f64::MAX);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(-f64::INFINITY);
                    assert_eq!(format!("{}", f), "-inf");
                    let f = GenericFraction::<$t>::from(f64::INFINITY);
                    assert_eq!(format!("{}", f), "inf");
                    let f = GenericFraction::<$t>::from(-0.0_f64);
                    assert_eq!(format!("{}", f), "0");
                    let f = GenericFraction::<$t>::from(0.0_f64);
                    assert_eq!(format!("{}", f), "0");
                    let f = GenericFraction::<$t>::from(-1.0_f64);
                    assert_eq!(format!("{}", f), "-1");
                    let f = GenericFraction::<$t>::from(1.0_f64);
                    assert_eq!(format!("{}", f), "1");
                    // Arbitrary tests
                    assert_eq!(format!("{}", f), "1");
                    let f = GenericFraction::<$t>::from(2.0);
                    assert_eq!(format!("{}", f), "2");
                    let f = GenericFraction::<$t>::from(0.5);
                    assert_eq!(format!("{}", f), "1/2");
                    let f = GenericFraction::<$t>::from(15978.649);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(-0.75);
                    assert_eq!(format!("{}", f), "-3/4");
                )*
            };
        }

        macro_rules! test_for_larger_t {
            ( $($t:ty),*) => {
                $(
                    // f32 tests
                    let f = GenericFraction::<$t>::from(-f32::NAN);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(f32::NAN);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(-f32::MIN);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(f32::MIN);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(-f32::MAX);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(f32::MAX);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(-f32::INFINITY);
                    assert_eq!(format!("{}", f), "-inf");
                    let f = GenericFraction::<$t>::from(f32::INFINITY);
                    assert_eq!(format!("{}", f), "inf");
                    let f = GenericFraction::<$t>::from(-0.0_f32);
                    assert_eq!(format!("{}", f), "0");
                    let f = GenericFraction::<$t>::from(0.0_f32);
                    assert_eq!(format!("{}", f), "0");
                    let f = GenericFraction::<$t>::from(-1.0_f32);
                    assert_eq!(format!("{}", f), "-1");
                    let f = GenericFraction::<$t>::from(1.0_f32);
                    assert_eq!(format!("{}", f), "1");
                    // f64 tests
                    let f = GenericFraction::<$t>::from(-f64::NAN);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(f64::NAN);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(-f64::MIN);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(f64::MIN);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(-f64::MAX);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(f64::MAX);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(-f64::INFINITY);
                    assert_eq!(format!("{}", f), "-inf");
                    let f = GenericFraction::<$t>::from(f64::INFINITY);
                    assert_eq!(format!("{}", f), "inf");
                    let f = GenericFraction::<$t>::from(-0.0_f64);
                    assert_eq!(format!("{}", f), "0");
                    let f = GenericFraction::<$t>::from(0.0_f64);
                    assert_eq!(format!("{}", f), "0");
                    let f = GenericFraction::<$t>::from(-1.0_f64);
                    assert_eq!(format!("{}", f), "-1");
                    let f = GenericFraction::<$t>::from(1.0_f64);
                    assert_eq!(format!("{}", f), "1");
                    // Arbitrary tests
                    let f = GenericFraction::<$t>::from(2.0);
                    assert_eq!(format!("{}", f), "2");
                    let f = GenericFraction::<$t>::from(0.5);
                    assert_eq!(format!("{}", f), "1/2");
                    let f = GenericFraction::<$t>::from(15978.649);
                    assert_eq!(format!("{}", f), "15978649/1000");
                    let f = GenericFraction::<$t>::from(-0.75);
                    assert_eq!(format!("{}", f), "-3/4");
                )*
            };
        }

        macro_rules! test_for_big_t {
            ( $($t:ty),*) => {
                $(
                    // Note: we don't test min/max for big_t because the value depends on the type
                    // f32 tests
                    let f = GenericFraction::<$t>::from(-f32::NAN);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(f32::NAN);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(-f32::INFINITY);
                    assert_eq!(format!("{}", f), "-inf");
                    let f = GenericFraction::<$t>::from(f32::INFINITY);
                    assert_eq!(format!("{}", f), "inf");
                    let f = GenericFraction::<$t>::from(-0.0_f32);
                    assert_eq!(format!("{}", f), "0");
                    let f = GenericFraction::<$t>::from(0.0_f32);
                    assert_eq!(format!("{}", f), "0");
                    let f = GenericFraction::<$t>::from(-1.0_f32);
                    assert_eq!(format!("{}", f), "-1");
                    let f = GenericFraction::<$t>::from(1.0_f32);
                    assert_eq!(format!("{}", f), "1");
                    // f64 tests
                    let f = GenericFraction::<$t>::from(-f64::NAN);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(f64::NAN);
                    assert_eq!(format!("{}", f), "NaN");
                    let f = GenericFraction::<$t>::from(-f64::INFINITY);
                    assert_eq!(format!("{}", f), "-inf");
                    let f = GenericFraction::<$t>::from(f64::INFINITY);
                    assert_eq!(format!("{}", f), "inf");
                    let f = GenericFraction::<$t>::from(-0.0_f64);
                    assert_eq!(format!("{}", f), "0");
                    let f = GenericFraction::<$t>::from(0.0_f64);
                    assert_eq!(format!("{}", f), "0");
                    let f = GenericFraction::<$t>::from(-1.0_f64);
                    assert_eq!(format!("{}", f), "-1");
                    let f = GenericFraction::<$t>::from(1.0_f64);
                    assert_eq!(format!("{}", f), "1");
                    // Arbitrary tests
                    let f = GenericFraction::<$t>::from(2.0);
                    assert_eq!(format!("{}", f), "2");
                    let f = GenericFraction::<$t>::from(0.5);
                    assert_eq!(format!("{}", f), "1/2");
                    let f = GenericFraction::<$t>::from(15978.649);
                    assert_eq!(format!("{}", f), "15978649/1000");
                    let f = GenericFraction::<$t>::from(-0.75);
                    assert_eq!(format!("{}", f), "-3/4");
                    let f = GenericFraction::<$t>::from(-0.5);
                    assert_eq!(format!("{}", f), "-1/2");
                )*
            };
        }

        test_for_smaller_t!(u8, i8, u16, i16);
        test_for_larger_t!(u32, i32, u64, i64, usize, isize);
        test_for_big_t!(u128, i128);

        #[cfg(feature = "with-bigint")]
        {
            use crate::{BigInt, BigUint};
            test_for_big_t!(BigUint, BigInt);
        }
    }

    #[test]
    fn fraction_test_default() {
        let fra = Frac::default();
        assert_eq!(fra.numer(), Some(&0u8));
        assert_eq!(fra.denom(), Some(&1u8));

        #[cfg(feature = "with-bigint")]
        {
            let fra = BigFraction::default();
            assert_eq!(fra.numer(), Some(&BigUint::from(0u8)));
            assert_eq!(fra.denom(), Some(&BigUint::from(1u8)));
        }
    }

    fn clamp_agree_with_cmp<T: Clone + Integer + std::fmt::Debug + GenericInteger>(
        min: &GenericFraction<T>,
        max: &GenericFraction<T>,
        test_value: &GenericFraction<T>,
    ) {
        if min.cmp(max) == Ordering::Greater {
            panic!("min is greater than max");
        }

        let clamped = test_value.clamp(min, max);

        match (test_value.cmp(min), test_value.cmp(max)) {
            (Ordering::Less, Ordering::Less) => assert_eq!(clamped, min),
            (Ordering::Less, Ordering::Equal) => assert_eq!(clamped, min),
            (Ordering::Less, Ordering::Greater) => {
                panic!("Shouldn't be possible to be less than min and greater than max")
            }

            (Ordering::Equal, Ordering::Less) => assert_eq!(clamped, min),
            (Ordering::Equal, Ordering::Equal) => {
                assert_eq!(clamped, min);
                assert_eq!(clamped, max);
            }
            (Ordering::Equal, Ordering::Greater) => assert_eq!(clamped, max),

            (Ordering::Greater, Ordering::Less) => assert_eq!(clamped, test_value),
            (Ordering::Greater, Ordering::Equal) => assert_eq!(clamped, max),
            (Ordering::Greater, Ordering::Greater) => assert_eq!(clamped, max),
        }
    }
    #[test]
    fn rational_rational_rational_clamp_cmp_agreement_test() {
        let mut rng = rand::thread_rng();
        for _ in 0..10 {
            let base_value: i8 = rng.gen();
            let min = GenericFraction::<i64>::from(base_value);

            let bump: u8 = rng.gen();
            let bump: i32 = base_value as i32 + bump as i32;
            let max = GenericFraction::<i64>::from(bump);

            let test_value = GenericFraction::<i64>::new(rng.gen::<u8>(), rng.gen::<u8>());

            clamp_agree_with_cmp(&min, &max, &test_value);
        }
    }
    #[test]
    fn rational_nan_rational_clamp_cmp_agreement_test() {
        let mut rng = rand::thread_rng();
        for _ in 0..10 {
            let base_value: i8 = rng.gen();
            let min = GenericFraction::<i64>::from(base_value);

            let bump: u8 = rng.gen();
            let bump: i32 = base_value as i32 + bump as i32;
            let max = GenericFraction::<i64>::from(bump);

            let nan = GenericFraction::<i64>::NaN;

            clamp_agree_with_cmp(&min, &max, &nan);
        }
    }
    #[test]
    fn nan_pos_infinity_partail_cmp_cmp_agreement_test() {
        let nan = GenericFraction::<i64>::NaN;
        let pos_inf = GenericFraction::<i64>::infinity();

        assert_eq!(nan.partial_cmp(&pos_inf), Some(nan.cmp(&pos_inf)));
    }
    #[test]
    fn nan_neg_infinity_partail_cmp_cmp_agreement_test() {
        let nan = GenericFraction::<i64>::NaN;
        let neg_inf = GenericFraction::<i64>::neg_infinity();

        assert_eq!(nan.partial_cmp(&neg_inf), Some(nan.cmp(&neg_inf)));
    }
    #[test]
    fn nan_rational_partail_cmp_cmp_agreement_test() {
        let mut rng = rand::thread_rng();

        let nan = GenericFraction::<i64>::NaN;

        for _ in 0..10 {
            let base_value: i8 = rng.gen();
            let test_value = GenericFraction::<i64>::from(base_value);

            assert_eq!(nan.partial_cmp(&test_value), Some(nan.cmp(&test_value)));
        }
    }
    #[test]
    fn pos_inf_rational_partail_cmp_cmp_agreement_test() {
        let mut rng = rand::thread_rng();

        let pos_inf = GenericFraction::<i64>::infinity();
        for _ in 0..10 {
            let base_value: i8 = rng.gen();
            let test_value = GenericFraction::<i64>::from(base_value);

            assert_eq!(
                pos_inf.partial_cmp(&test_value),
                Some(pos_inf.cmp(&test_value))
            );
        }
    }
    #[test]
    fn neg_inf_rational_partail_cmp_cmp_agreement_test() {
        let mut rng = rand::thread_rng();

        let neg_inf = GenericFraction::<i64>::neg_infinity();
        for _ in 0..10 {
            let base_value: i8 = rng.gen();
            let test_value = GenericFraction::<i64>::from(base_value);

            assert_eq!(
                neg_inf.partial_cmp(&test_value),
                Some(neg_inf.cmp(&test_value))
            );
        }
    }

    #[test]
    fn constant_one() {
        let constant_one = <Frac as ConstOne>::ONE;
        let one = Frac::one();
        assert_eq!(constant_one, one);
    }

    #[test]
    fn constant_zero() {
        let constant_zero = <Frac as ConstZero>::ZERO;
        let zero = Frac::zero();
        assert_eq!(constant_zero, zero);
    }

    #[test]
    fn consistency_partial_cmp() {
        let nan = Frac::nan();
        let neg_inf = Frac::neg_infinity();
        let neg_one = Frac::one() * -1;
        let zero = Frac::zero();
        let one = Frac::one();
        let inf = Frac::infinity();

        // 1. a == b if and only if partial_cmp(a, b) == Some(Equal).

        assert_eq!(
            Some(Ordering::Equal),
            GenericFraction::partial_cmp(&nan, &nan)
        );
        assert_eq!(
            Some(Ordering::Equal),
            GenericFraction::partial_cmp(&neg_inf, &neg_inf)
        );
        assert_eq!(
            Some(Ordering::Equal),
            GenericFraction::partial_cmp(&neg_one, &neg_one)
        );
        assert_eq!(
            Some(Ordering::Equal),
            GenericFraction::partial_cmp(&zero, &zero)
        );
        assert_eq!(
            Some(Ordering::Equal),
            GenericFraction::partial_cmp(&one, &one)
        );
        assert_eq!(
            Some(Ordering::Equal),
            GenericFraction::partial_cmp(&inf, &inf)
        );

        // 2. a < b if and only if partial_cmp(a, b) == Some(Less)

        assert_eq!(
            Some(Ordering::Less),
            GenericFraction::partial_cmp(&nan, &neg_inf)
        );
        assert_eq!(
            Some(Ordering::Less),
            GenericFraction::partial_cmp(&nan, &neg_one)
        );
        assert_eq!(
            Some(Ordering::Less),
            GenericFraction::partial_cmp(&nan, &zero)
        );
        assert_eq!(
            Some(Ordering::Less),
            GenericFraction::partial_cmp(&nan, &one)
        );
        assert_eq!(
            Some(Ordering::Less),
            GenericFraction::partial_cmp(&nan, &inf)
        );

        assert_eq!(
            Some(Ordering::Less),
            GenericFraction::partial_cmp(&neg_inf, &neg_one)
        );
        assert_eq!(
            Some(Ordering::Less),
            GenericFraction::partial_cmp(&neg_inf, &zero)
        );
        assert_eq!(
            Some(Ordering::Less),
            GenericFraction::partial_cmp(&neg_inf, &one)
        );
        assert_eq!(
            Some(Ordering::Less),
            GenericFraction::partial_cmp(&neg_inf, &inf)
        );

        assert_eq!(
            Some(Ordering::Less),
            GenericFraction::partial_cmp(&neg_one, &zero)
        );
        assert_eq!(
            Some(Ordering::Less),
            GenericFraction::partial_cmp(&neg_one, &one)
        );
        assert_eq!(
            Some(Ordering::Less),
            GenericFraction::partial_cmp(&neg_one, &inf)
        );

        assert_eq!(
            Some(Ordering::Less),
            GenericFraction::partial_cmp(&zero, &one)
        );
        assert_eq!(
            Some(Ordering::Less),
            GenericFraction::partial_cmp(&zero, &inf)
        );

        assert_eq!(
            Some(Ordering::Less),
            GenericFraction::partial_cmp(&one, &inf)
        );

        // 3. a > b if and only if partial_cmp(a, b) == Some(Greater)

        assert_eq!(
            Some(Ordering::Greater),
            GenericFraction::partial_cmp(&neg_inf, &nan)
        );

        assert_eq!(
            Some(Ordering::Greater),
            GenericFraction::partial_cmp(&neg_one, &neg_inf)
        );
        assert_eq!(
            Some(Ordering::Greater),
            GenericFraction::partial_cmp(&neg_one, &nan)
        );

        assert_eq!(
            Some(Ordering::Greater),
            GenericFraction::partial_cmp(&zero, &nan)
        );
        assert_eq!(
            Some(Ordering::Greater),
            GenericFraction::partial_cmp(&zero, &neg_inf)
        );
        assert_eq!(
            Some(Ordering::Greater),
            GenericFraction::partial_cmp(&zero, &neg_one)
        );

        assert_eq!(
            Some(Ordering::Greater),
            GenericFraction::partial_cmp(&one, &nan)
        );
        assert_eq!(
            Some(Ordering::Greater),
            GenericFraction::partial_cmp(&one, &neg_inf)
        );
        assert_eq!(
            Some(Ordering::Greater),
            GenericFraction::partial_cmp(&one, &neg_one)
        );
        assert_eq!(
            Some(Ordering::Greater),
            GenericFraction::partial_cmp(&one, &zero)
        );

        assert_eq!(
            Some(Ordering::Greater),
            GenericFraction::partial_cmp(&inf, &nan)
        );
        assert_eq!(
            Some(Ordering::Greater),
            GenericFraction::partial_cmp(&inf, &neg_inf)
        );
        assert_eq!(
            Some(Ordering::Greater),
            GenericFraction::partial_cmp(&inf, &neg_one)
        );
        assert_eq!(
            Some(Ordering::Greater),
            GenericFraction::partial_cmp(&inf, &zero)
        );
        assert_eq!(
            Some(Ordering::Greater),
            GenericFraction::partial_cmp(&inf, &one)
        );
    }

    #[test]
    fn consistency_cmp() {
        let nan = Frac::nan();
        let neg_inf = Frac::neg_infinity();
        let neg_one = Frac::one() * -1;
        let zero = Frac::zero();
        let one = Frac::one();
        let inf = Frac::infinity();

        // 1. a == b if and only if partial_cmp(a, b) == Some(Equal).

        assert_eq!(Ordering::Equal, GenericFraction::cmp(&nan, &nan));
        assert_eq!(Ordering::Equal, GenericFraction::cmp(&neg_inf, &neg_inf));
        assert_eq!(Ordering::Equal, GenericFraction::cmp(&neg_one, &neg_one));
        assert_eq!(Ordering::Equal, GenericFraction::cmp(&zero, &zero));
        assert_eq!(Ordering::Equal, GenericFraction::cmp(&one, &one));
        assert_eq!(Ordering::Equal, GenericFraction::cmp(&inf, &inf));

        // 2. a < b if and only if partial_cmp(a, b) == Some(Less)

        assert_eq!(Ordering::Less, GenericFraction::cmp(&nan, &neg_inf));
        assert_eq!(Ordering::Less, GenericFraction::cmp(&nan, &neg_one));
        assert_eq!(Ordering::Less, GenericFraction::cmp(&nan, &zero));
        assert_eq!(Ordering::Less, GenericFraction::cmp(&nan, &one));
        assert_eq!(Ordering::Less, GenericFraction::cmp(&nan, &inf));

        assert_eq!(Ordering::Less, GenericFraction::cmp(&neg_inf, &neg_one));
        assert_eq!(Ordering::Less, GenericFraction::cmp(&neg_inf, &zero));
        assert_eq!(Ordering::Less, GenericFraction::cmp(&neg_inf, &one));
        assert_eq!(Ordering::Less, GenericFraction::cmp(&neg_inf, &inf));

        assert_eq!(Ordering::Less, GenericFraction::cmp(&neg_one, &zero));
        assert_eq!(Ordering::Less, GenericFraction::cmp(&neg_one, &one));
        assert_eq!(Ordering::Less, GenericFraction::cmp(&neg_one, &inf));

        assert_eq!(Ordering::Less, GenericFraction::cmp(&zero, &one));
        assert_eq!(Ordering::Less, GenericFraction::cmp(&zero, &inf));

        assert_eq!(Ordering::Less, GenericFraction::cmp(&one, &inf));

        // 3. a > b if and only if partial_cmp(a, b) == Some(Greater)

        assert_eq!(Ordering::Greater, GenericFraction::cmp(&neg_inf, &nan));

        assert_eq!(Ordering::Greater, GenericFraction::cmp(&neg_one, &neg_inf));
        assert_eq!(Ordering::Greater, GenericFraction::cmp(&neg_one, &nan));

        assert_eq!(Ordering::Greater, GenericFraction::cmp(&zero, &nan));
        assert_eq!(Ordering::Greater, GenericFraction::cmp(&zero, &neg_inf));
        assert_eq!(Ordering::Greater, GenericFraction::cmp(&zero, &neg_one));

        assert_eq!(Ordering::Greater, GenericFraction::cmp(&one, &nan));
        assert_eq!(Ordering::Greater, GenericFraction::cmp(&one, &neg_inf));
        assert_eq!(Ordering::Greater, GenericFraction::cmp(&one, &neg_one));
        assert_eq!(Ordering::Greater, GenericFraction::cmp(&one, &zero));

        assert_eq!(Ordering::Greater, GenericFraction::cmp(&inf, &nan));
        assert_eq!(Ordering::Greater, GenericFraction::cmp(&inf, &neg_inf));
        assert_eq!(Ordering::Greater, GenericFraction::cmp(&inf, &neg_one));
        assert_eq!(Ordering::Greater, GenericFraction::cmp(&inf, &zero));
        assert_eq!(Ordering::Greater, GenericFraction::cmp(&inf, &one));
    }

    #[test]
    fn consistency_eq() {
        let nan = Frac::nan();
        let neg_inf = Frac::neg_infinity();
        let neg_one = Frac::one() * -1;
        let zero = Frac::zero();
        let one = Frac::one();
        let inf = Frac::infinity();

        assert!(GenericFraction::eq(&nan, &nan));
        assert!(!GenericFraction::eq(&nan, &neg_inf));
        assert!(!GenericFraction::eq(&nan, &neg_one));
        assert!(!GenericFraction::eq(&nan, &zero));
        assert!(!GenericFraction::eq(&nan, &one));
        assert!(!GenericFraction::eq(&nan, &inf));

        assert!(GenericFraction::eq(&neg_inf, &neg_inf));
        assert!(!GenericFraction::eq(&neg_inf, &neg_one));
        assert!(!GenericFraction::eq(&neg_inf, &zero));
        assert!(!GenericFraction::eq(&neg_inf, &one));
        assert!(!GenericFraction::eq(&neg_inf, &inf));

        assert!(GenericFraction::eq(&neg_one, &neg_one));
        assert!(!GenericFraction::eq(&neg_one, &zero));
        assert!(!GenericFraction::eq(&neg_one, &one));
        assert!(!GenericFraction::eq(&neg_one, &inf));

        assert!(GenericFraction::eq(&zero, &zero));
        assert!(!GenericFraction::eq(&zero, &one));
        assert!(!GenericFraction::eq(&zero, &inf));

        assert!(GenericFraction::eq(&one, &one));
        assert!(!GenericFraction::eq(&one, &inf));

        assert!(GenericFraction::eq(&inf, &inf));

        assert!(!GenericFraction::ne(&nan, &nan));
        assert!(GenericFraction::ne(&nan, &neg_inf));
        assert!(GenericFraction::ne(&nan, &neg_one));
        assert!(GenericFraction::ne(&nan, &zero));
        assert!(GenericFraction::ne(&nan, &one));
        assert!(GenericFraction::ne(&nan, &inf));

        assert!(!GenericFraction::ne(&neg_inf, &neg_inf));
        assert!(GenericFraction::ne(&neg_inf, &neg_one));
        assert!(GenericFraction::ne(&neg_inf, &zero));
        assert!(GenericFraction::ne(&neg_inf, &one));
        assert!(GenericFraction::ne(&neg_inf, &inf));

        assert!(!GenericFraction::ne(&neg_one, &neg_one));
        assert!(GenericFraction::ne(&neg_one, &zero));
        assert!(GenericFraction::ne(&neg_one, &one));
        assert!(GenericFraction::ne(&neg_one, &inf));

        assert!(!GenericFraction::ne(&zero, &zero));
        assert!(GenericFraction::ne(&zero, &one));
        assert!(GenericFraction::ne(&zero, &inf));

        assert!(!GenericFraction::ne(&one, &one));
        assert!(GenericFraction::ne(&one, &inf));

        assert!(!GenericFraction::ne(&inf, &inf));
    }
}
