#[cfg(test)]
mod tests {
    #[cfg(feature = "with-bigint")]
    use super::BigUint;
    use super::{
        CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, DynaInt, GenericInteger, Integer, Num, One,
        Sign, ToPrimitive, Zero,
    };
    use crate::error::ParseError;

    type D = DynaInt<u8, u16>;

    generate_ops_tests!(
        Zero => {D::zero()};
        One => {D::one()};
        Two => {D::from(2u8)};
        Three => {D::from(3u8)};
        Four => {D::from(4u8)};
    );

    #[test]
    fn growth() {
        let m8 = u8::MAX;

        let mut val = D::from(m8);

        assert_eq!(m8, val.clone().unpack().ok().unwrap());

        val += D::zero();

        assert_eq!(m8, val.clone().unpack().ok().unwrap());

        val += D::one();

        assert_eq!(u16::from(m8) + 1, val.clone().unpack().err().unwrap());

        val += D::one();

        assert_eq!(u16::from(m8) + 2, val.clone().unpack().err().unwrap());

        val -= D::from(2u8);

        assert_eq!(m8, val.clone().unpack().ok().unwrap());
    }

    #[test]
    fn to_primitive() {
        {
            let mut val = D::from(255u8);

            assert_eq!(Some(255u64), val.to_u64());
            val += D::one();
            assert_eq!(Some(256u64), val.to_u64());
        }

        {
            let mut val = D::from(255u8);

            assert_eq!(Some(255i64), val.to_i64());
            val += D::one();
            assert_eq!(Some(256i64), val.to_i64());
        }
    }

    #[test]
    fn generic_integer() {
        assert_eq!(0u8, D::_0().unpack().ok().unwrap());
        assert_eq!(1u8, D::_1().unpack().ok().unwrap());
        assert_eq!(10u8, D::_10().unpack().ok().unwrap());

        assert!(D::_0r().is_none());
        assert!(D::_1r().is_none());
        assert!(D::_10r().is_none());

        let (s, v) = D::from(254u8).get_signed_value();
        assert_eq!(s, Sign::Plus);
        assert_eq!(254u8, v.unpack().ok().unwrap());
    }

    #[test]
    fn partial_eq() {
        assert_ne!(D::from(1u8), D::from(256u16));
        assert_eq!(D::from(256u16), D::from(256u16));
        assert_ne!(D::from(256u16), D::from(257u16));
    }

    #[test]
    fn add_assign() {
        {
            let mut val = D::from(255u8);
            val += D::from(256u16);

            assert_eq!(511u16, val.unpack().err().unwrap());
        }

        {
            let mut val = D::from(255u8);
            val += &D::from(256u16);

            assert_eq!(511u16, val.unpack().err().unwrap());
        }

        {
            let mut val = D::from(256u16);
            val += &D::one();

            assert_eq!(257u16, val.unpack().err().unwrap());
        }

        {
            let mut val = D::from(255u8);
            val += &D::from(255u8);

            assert_eq!(510u16, val.unpack().err().unwrap());
        }

        {
            let mut val = D::from(256u16);
            val += D::from(256u16);

            assert_eq!(512u16, val.unpack().err().unwrap());
        }

        {
            let mut val = D::from(256u16);
            val += &D::from(256u16);

            assert_eq!(512u16, val.unpack().err().unwrap());
        }
    }

    #[test]
    fn op_add() {
        assert_eq!(2u8, (D::one() + &D::one()).unpack().ok().unwrap());

        assert_eq!(257u16, (D::one() + D::from(256u16)).unpack().err().unwrap());
        assert_eq!(256u16, (D::one() + &D::from(255u8)).unpack().err().unwrap());
        assert_eq!(
            257u16,
            (D::one() + &D::from(256u16)).unpack().err().unwrap()
        );
        assert_eq!(
            257u16,
            (&D::one() + &D::from(256u16)).unpack().err().unwrap()
        );

        assert_eq!(257u16, (D::from(256u16) + D::one()).unpack().err().unwrap());
        assert_eq!(
            257u16,
            (D::from(256u16) + &D::one()).unpack().err().unwrap()
        );
        assert_eq!(
            257u16,
            (&D::from(256u16) + &D::one()).unpack().err().unwrap()
        );

        assert_eq!(
            512u16,
            (D::from(256u16) + D::from(256u16)).unpack().err().unwrap()
        );
        assert_eq!(
            512u16,
            (D::from(256u16) + &D::from(256u16)).unpack().err().unwrap()
        );
        assert_eq!(
            512u16,
            (&D::from(256u16) + &D::from(256u16))
                .unpack()
                .err()
                .unwrap()
        );
    }

    #[test]
    fn op_bit_and() {
        assert_eq!(D::one(), D::one() & D::one());
        assert_eq!(D::one(), &D::one() & &D::one());

        assert_eq!(D::zero(), D::one() & D::from(256u16));
        assert_eq!(D::zero(), &D::one() & &D::from(256u16));

        assert_eq!(D::zero(), D::from(256u16) & D::one());
        assert_eq!(D::zero(), &D::from(256u16) & &D::one());

        assert_eq!(D::from(256u16), D::from(256u16) & D::from(257u16));
        assert_eq!(D::from(256u16), &D::from(256u16) & &D::from(257u16));

        {
            let mut v = D::one();
            v &= D::one();
            assert_eq!(v, D::one());
        }

        {
            let mut v = D::one();
            v &= &D::one();
            assert_eq!(v, D::one());
        }

        {
            let mut v = D::one();
            v &= D::from(256u16);
            assert_eq!(v, D::zero());
        }

        {
            let mut v = D::one();
            v &= &D::from(256u16);
            assert_eq!(v, D::zero());
        }

        {
            let mut v = D::from(256u16);
            v &= D::one();
            assert_eq!(v, D::zero());
        }

        {
            let mut v = D::from(256u16);
            v &= &D::one();
            assert_eq!(v, D::zero());
        }

        {
            let mut v = D::from(256u16);
            v &= D::from(257u16);
            assert_eq!(v, D::from(256u16));
        }

        {
            let mut v = D::from(256u16);
            v &= &D::from(257u16);
            assert_eq!(v, D::from(256u16));
        }
    }

    #[test]
    fn op_rem() {
        assert_eq!(D::one(), D::from(3u8) % D::from(2u8));
        assert_eq!(D::one(), D::from(3u8) % &D::from(2u8));
        assert_eq!(D::one(), &D::from(3u8) % &D::from(2u8));

        {
            let mut v = D::from(3u8);
            v %= D::from(2u8);
            assert_eq!(v, D::one());
        }

        {
            let mut v = D::from(3u8);
            v %= &D::from(2u8);
            assert_eq!(v, D::one());
        }

        assert_eq!(D::one(), D::one() % D::from(256u16));
        assert_eq!(D::one(), D::one() % &D::from(256u16));
        assert_eq!(D::one(), &D::one() % &D::from(256u16));

        {
            let mut v = D::from(D::one());
            v %= D::from(256u16);
            assert_eq!(v, D::one());
        }

        {
            let mut v = D::from(D::one());
            v %= &D::from(256u16);
            assert_eq!(v, D::one());
        }

        assert_eq!(D::zero(), D::from(256u16) % D::one());
        assert_eq!(D::zero(), D::from(256u16) % &D::one());
        assert_eq!(D::zero(), &D::from(256u16) % &D::one());

        {
            let mut v = D::from(256u16);
            v %= D::one();
            assert_eq!(v, D::zero());
        }

        {
            let mut v = D::from(256u16);
            v %= &D::one();
            assert_eq!(v, D::zero());
        }

        assert_eq!(D::zero(), D::from(256u16) % D::from(256u16));
        assert_eq!(D::zero(), D::from(256u16) % &D::from(256u16));
        assert_eq!(D::zero(), &D::from(256u16) % &D::from(256u16));

        {
            let mut v = D::from(D::from(256u16));
            v %= D::from(256u16);
            assert_eq!(v, D::zero());
        }

        {
            let mut v = D::from(D::from(256u16));
            v %= &D::from(256u16);
            assert_eq!(v, D::zero());
        }
    }

    #[test]
    fn checked_math() {
        assert_eq!(Some(D::zero()), D::zero().checked_add(&D::zero()));
        assert_eq!(Some(D::from(256u16)), D::one().checked_add(&D::from(255u8)));
        assert_eq!(
            Some(D::from(257u16)),
            D::one().checked_add(&D::from(256u16))
        );

        assert_eq!(
            Some(D::from(257u16)),
            D::from(256u16).checked_add(&D::one())
        );
        assert_eq!(
            Some(D::from(512u16)),
            D::from(256u16).checked_add(&D::from(256u16))
        );

        assert_eq!(None, D::from(u16::MAX).checked_add(&D::one()));
        assert_eq!(None, D::one().checked_add(&D::from(u16::MAX)));
    }

    #[test]
    fn from_uint() {
        type D = DynaInt<u16, u32>;

        assert_eq!(D::one(), D::from(1u16));
    }

    #[cfg(feature = "with-bigint")]
    #[test]
    fn from_biguint() {
        use super::BigUint;

        type D = DynaInt<u8, BigUint>;

        assert_eq!(D::one(), D::from(BigUint::one()));
    }

    #[test]
    fn zero() {
        assert!(D::zero().is_zero());
    }

    #[test]
    fn from_str_radix() {
        let parsed: Result<D, ParseError> = D::from_str_radix("256", 10);
        assert_eq!(Ok(D::from(256u16)), parsed);
        assert_eq!(D::from(1u8), D::from_str_radix("1", 10).unwrap());
        assert_eq!(D::from(2u8), D::from_str_radix("10", 2).unwrap());
        assert_eq!(D::from(35u8), D::from_str_radix("z", 36).unwrap());
        assert_eq!(D::from(256u16), D::from_str_radix("100000000", 2).unwrap());
        assert_eq!(Err(ParseError::ParseIntError), D::from_str_radix("z", 10));
        assert_eq!(
            Err(ParseError::ParseIntError),
            D::from_str_radix("65536", 10)
        );

        for radix in [0, 1, 37, u32::MAX] {
            assert_eq!(
                Err(ParseError::UnsupportedBase),
                D::from_str_radix("1", radix)
            );
        }
    }

    #[cfg(feature = "with-bigint")]
    #[test]
    fn from_str_radix_rejects_invalid_bases_for_biguint() {
        type BigD = DynaInt<u8, BigUint>;

        for radix in [0, 37] {
            assert_eq!(
                Err(ParseError::UnsupportedBase),
                BigD::from_str_radix("1", radix)
            );
        }
    }

    #[test]
    fn integer() {
        assert_eq!(D::from(127u8), D::from(255u8).div_floor(&D::from(2u8)));
        assert_eq!(D::zero(), D::one().div_floor(&D::from(256u16)));

        assert_eq!(D::from(128u8), D::from(256u16).div_floor(&D::from(2u8)));
        assert_eq!(D::one(), D::from(257u16).div_floor(&D::from(256u16)));

        assert!(D::one().is_multiple_of(&D::one()));
        assert!(!D::one().is_multiple_of(&D::from(257u16)));
        assert!(D::from(257u16).is_multiple_of(&D::one()));
        assert!(!D::from(257u16).is_multiple_of(&D::from(256u16)));

        assert!(D::one().is_odd());
        assert!(D::from(256u16).is_even());

        assert_eq!((D::one(), D::zero()), D::one().div_rem(&D::one()));
        assert_eq!((D::zero(), D::one()), D::one().div_rem(&D::from(256u16)));
        assert_eq!(
            (D::from(256u16), D::zero()),
            D::from(256u16).div_rem(&D::one())
        );
        assert_eq!(
            (D::one(), D::one()),
            D::from(257u16).div_rem(&D::from(256u16))
        );
    }
}
