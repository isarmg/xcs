#[cfg(test)]
mod tests {
    use crate::error::ParseError;
    use std::cmp::Ordering;
    use std::collections::{BTreeSet, HashSet};
    use {CheckedAdd, CheckedDiv, CheckedMul, CheckedSub};

    use super::{GenericDecimal, One};
    use fraction::GenericFraction;
    use prelude::Decimal;
    use std::hash::{Hash, Hasher};
    use std::str::FromStr;

    type D = GenericDecimal<u8, u8>;

    fn hash_it(target: &impl Hash) -> u64 {
        use std::collections::hash_map::DefaultHasher;

        let mut h = DefaultHasher::new();
        target.hash(&mut h);
        h.finish()
    }

    generate_ops_tests! (
        NaN => {D::nan()};
        NegInf => {D::neg_infinity()};
        PosInf => {D::infinity()};
        Zero => {D::from(0)};
        Half => {D::from(0.5)};
        One => {D::from(1)};
        Two => {D::from(2)};
        Three => {D::from(3)};
        Four => {D::from(4)};
    );

    #[test]
    fn hash_and_partial_eq() {
        {
            let one = Decimal::from(152.568);
            let two = Decimal::from(328.76842);

            let div = two / one.set_precision(16);
            let red = Decimal::from("2.1548976194221592");

            assert_eq!(div.get_precision(), 16);
            assert_eq!(div, red);
            assert_eq!(hash_it(&div), hash_it(&red));
        }

        {
            let one = Decimal::from(152.568);
            let two = Decimal::from(328.76842);

            let mul = one * two;

            assert_eq!(mul.get_precision(), 5);
            assert_eq!(mul, Decimal::from("50159.5403"));
            assert_eq!(hash_it(&mul), hash_it(&Decimal::from("50159.5403")));
            assert_eq!(mul.set_precision(6), Decimal::from("50159.540302"));
            assert_eq!(
                hash_it(&mul.set_precision(6)),
                hash_it(&Decimal::from("50159.540302"))
            );
        }
    }

    #[test]
    fn comparison_reported_bug_pair() {
        let a = Decimal::from_str("0.5").unwrap() / Decimal::from_str("0.3").unwrap();
        let b = Decimal::from_str("1.6").unwrap();

        assert_eq!(a, b);
        assert_eq!(Some(Ordering::Equal), a.partial_cmp(&b));
        assert_eq!(Ordering::Equal, a.cmp(&b));
        assert!(!(a < b));
        assert!(!(a > b));
        assert!(a <= b);
        assert!(a >= b);

        let mut set = BTreeSet::new();
        set.insert(a);
        set.insert(b);
        assert_eq!(set.len(), 1);

        let mut hash = HashSet::new();
        hash.insert(a);
        hash.insert(b);
        assert_eq!(hash.len(), 1);
        assert_eq!(hash_it(&a), hash_it(&b));

        assert_eq!(vec![a].binary_search(&b), Ok(0));
    }

    #[test]
    fn comparison_trailing_zeroes_and_precision() {
        let one = Decimal::from_str("1.0").unwrap();
        let one_with_more_zeroes = Decimal::from_str("1.000").unwrap();

        assert_eq!(one, one_with_more_zeroes);
        assert_eq!(hash_it(&one), hash_it(&one_with_more_zeroes));

        let mut set = BTreeSet::new();
        set.insert(one);
        set.insert(one_with_more_zeroes);
        assert_eq!(set.len(), 1);

        let mut hash = HashSet::new();
        hash.insert(one);
        hash.insert(one_with_more_zeroes);
        assert_eq!(hash.len(), 1);

        assert_eq!(vec![one].binary_search(&one_with_more_zeroes), Ok(0));
    }

    #[test]
    fn comparison_same_exact_fraction_different_precision() {
        type D = GenericDecimal<u64, u8>;

        let five_thirds_p1: D =
            GenericDecimal::from_fraction_with_precision(GenericFraction::new(5u64, 3u64), 1u8);
        let five_thirds_p2: D =
            GenericDecimal::from_fraction_with_precision(GenericFraction::new(5u64, 3u64), 2u8);

        assert!(five_thirds_p1 < five_thirds_p2);
        assert_eq!(five_thirds_p1, five_thirds_p1.set_precision(1));
        assert_eq!(five_thirds_p2, five_thirds_p2.set_precision(2));
    }

    #[test]
    fn comparison_truncation_p0() {
        let positive_one = Decimal::from_str("1.99").unwrap().set_precision(0);
        let positive_other = Decimal::from_str("1.01").unwrap().set_precision(0);
        let negative_one = Decimal::from_str("-1.99").unwrap().set_precision(0);
        let negative_other = Decimal::from_str("-1.01").unwrap().set_precision(0);

        assert_eq!(positive_one, positive_other);
        assert_eq!(negative_one, negative_other);
        assert!(!(positive_one < positive_other));
        assert!(!(negative_one < negative_other));
    }

    #[test]
    fn comparison_negative_and_zero() {
        use num::traits::Zero;

        let negative_zero = -Decimal::zero();
        let negative_zero_p0 = -Decimal::from_str("0.9").unwrap().set_precision(0);
        let negative_zero_p1 = -Decimal::from_str("0.04").unwrap().set_precision(1);

        assert_eq!(negative_zero, Decimal::from(0));
        assert_eq!(negative_zero_p0, Decimal::from(0));
        assert_eq!(negative_zero_p1, Decimal::from(0));
        assert_eq!(negative_zero, negative_zero_p0);
        assert_eq!(negative_zero, negative_zero_p1);
        assert_eq!(negative_zero_p0, negative_zero_p1);
        assert_eq!(hash_it(&negative_zero), hash_it(&negative_zero_p1));

        let mut set = BTreeSet::new();
        set.insert(negative_zero);
        set.insert(negative_zero_p0);
        set.insert(negative_zero_p1);
        assert_eq!(set.len(), 1);

        let mut set = HashSet::new();
        set.insert(negative_zero);
        set.insert(negative_zero_p0);
        set.insert(negative_zero_p1);
        assert_eq!(set.len(), 1);
        assert_eq!(hash_it(&negative_zero), hash_it(&negative_zero_p1));

        assert_eq!(vec![negative_zero].binary_search(&negative_zero_p1), Ok(0));
    }

    #[test]
    fn comparison_special_value_order() {
        let nan = Decimal::nan();
        let neg_inf = Decimal::neg_infinity();
        let finite = Decimal::from_str("1.6").unwrap();
        let inf = Decimal::infinity();

        assert_eq!(nan.cmp(&nan), Ordering::Equal);
        assert_eq!(nan.cmp(&neg_inf), Ordering::Less);
        assert_eq!(nan.cmp(&finite), Ordering::Less);
        assert_eq!(nan.cmp(&inf), Ordering::Less);

        assert_eq!(neg_inf.cmp(&finite), Ordering::Less);
        assert_eq!(neg_inf.cmp(&inf), Ordering::Less);

        assert_eq!(finite.cmp(&inf), Ordering::Less);
    }

    #[test]
    fn comparison_pairwise_eq_iff_cmp_equal() {
        let values = vec![
            Decimal::nan(),
            Decimal::neg_infinity(),
            (-Decimal::from_str("1").unwrap()),
            Decimal::from_str("-0.5").unwrap(),
            Decimal::from(0),
            Decimal::from_str("0.5").unwrap(),
            Decimal::from_str("1.6").unwrap(),
            Decimal::infinity(),
        ];

        for left in &values {
            for right in &values {
                let cmp = left.cmp(right);
                assert_eq!(left.eq(right), cmp == Ordering::Equal);
                assert_eq!(left.partial_cmp(right), Some(cmp));
            }
        }
    }

    #[test]
    fn comparison_reverse_antisymmetry() {
        let values = vec![
            Decimal::nan(),
            Decimal::neg_infinity(),
            (-Decimal::from_str("1").unwrap()),
            Decimal::from_str("-0.5").unwrap(),
            Decimal::from(0),
            Decimal::from_str("0.5").unwrap(),
            Decimal::from_str("1.6").unwrap(),
            Decimal::infinity(),
        ];

        for left in &values {
            for right in &values {
                let forward = left.cmp(right);
                let reverse = right.cmp(left);

                assert_eq!(forward, reverse.reverse());
            }
        }
    }

    #[test]
    fn comparison_transitivity_5_thirds_chain() {
        let five_thirds_p1 =
            Decimal::from_fraction_with_precision(GenericFraction::new(5u64, 3u64), 1);
        let one_dot_six = Decimal::from_str("1.6").unwrap();
        let one_dot_sixty_five = Decimal::from_str("1.65").unwrap();
        let negative_five_thirds_p1 = -five_thirds_p1;
        let negative_one_dot_six = -Decimal::from_str("1.6").unwrap();

        assert_eq!(five_thirds_p1, one_dot_six);
        assert_eq!(
            Some(Ordering::Equal),
            five_thirds_p1.partial_cmp(&one_dot_six)
        );
        assert!(one_dot_six < one_dot_sixty_five);
        assert!(five_thirds_p1 < one_dot_sixty_five);

        assert_eq!(negative_five_thirds_p1, negative_one_dot_six);
        assert_eq!(
            Some(Ordering::Equal),
            negative_five_thirds_p1.partial_cmp(&negative_one_dot_six)
        );
        assert_eq!(
            Ordering::Equal,
            negative_five_thirds_p1.cmp(&negative_one_dot_six)
        );
        assert!(!(negative_five_thirds_p1 < negative_one_dot_six));
        assert!(!(negative_five_thirds_p1 > negative_one_dot_six));
        assert!(negative_five_thirds_p1 <= negative_one_dot_six);
        assert!(negative_five_thirds_p1 >= negative_one_dot_six);
    }

    #[test]
    fn fmt_debug() {
        type F = GenericFraction<u64>;
        assert_eq!(
            format!("{:?}", Decimal::one()),
            format!("GenericDecimal(1 | prec=0; {:?}; 1)", F::one())
        );
    }

    #[test]
    fn summing_iterator() {
        let values = vec![Decimal::from(152.568), Decimal::from(328.76842)];
        let sum: Decimal = values.iter().sum();
        assert_eq!(sum, values[0] + values[1])
    }

    #[test]
    fn product_iterator() {
        let values = vec![Decimal::from(152.568), Decimal::from(328.76842)];
        let product: Decimal = values.iter().product();
        assert_eq!(product, values[0] * values[1])
    }

    #[test]
    fn calc_precision() {
        use super::BigUint;
        type BigDecimal = GenericDecimal<BigUint, usize>;

        let one = BigDecimal::from(1);
        let two = BigDecimal::from(2);
        let three = BigDecimal::from(3);
        let half = BigDecimal::from(1) / BigDecimal::from(2);
        let onethird = BigDecimal::from(1) / BigDecimal::from(3);

        assert_eq!(0, one.get_precision());
        assert_eq!(0, two.get_precision());
        assert_eq!(0, three.get_precision());
        assert_eq!(0, half.get_precision());
        assert_eq!(0, onethird.get_precision());
        assert_eq!(1, half.clone().calc_precision(None).get_precision());
        assert_eq!(0, half.clone().calc_precision(Some(0)).get_precision());
        assert_eq!(1, half.clone().calc_precision(Some(1)).get_precision());
        assert_eq!(1, half.clone().calc_precision(Some(255)).get_precision());

        assert_eq!(0, onethird.clone().calc_precision(Some(0)).get_precision());
        assert_eq!(1, onethird.clone().calc_precision(Some(1)).get_precision());
        assert_eq!(
            255,
            onethird.clone().calc_precision(Some(255)).get_precision()
        );
        assert_eq!(
            2056,
            onethird.clone().calc_precision(Some(2056)).get_precision()
        );

        type D = GenericDecimal<u64, u8>;
        let one = D::from(1);
        let two = D::from(2);
        let three = D::from(3);
        let half = one / two;
        let onethird = one / three;

        assert_eq!(0, one.get_precision());
        assert_eq!(0, two.get_precision());
        assert_eq!(0, three.get_precision());
        assert_eq!(0, half.get_precision());
        assert_eq!(0, onethird.get_precision());
        assert_eq!(1, half.calc_precision(None).get_precision());
        assert_eq!(255, onethird.calc_precision(None).get_precision());
    }

    #[test]
    fn decimal_test_default() {
        let dec = D::default();
        assert_eq!("0", format!("{}", dec));
        assert_eq!(0, dec.get_precision());

        #[cfg(feature = "with-bigint")]
        {
            use crate::BigDecimal;
            let dec = BigDecimal::default();
            assert_eq!("0", format!("{}", dec));
            assert_eq!(0, dec.get_precision());
        }
    }

    #[test]
    fn from_fraction_with_precision() {
        let one_third: GenericFraction<u64> = GenericFraction::new(1u64, 3u64);

        assert_eq!(
            GenericDecimal::<u64, u8>::from_fraction_with_precision(one_third, 18).get_precision(),
            18
        );
    }

    #[test]
    fn from_str_zero_denominator() {
        assert_eq!(Ok(Decimal::infinity()), Decimal::from_str("1/0"));
        assert_eq!(Ok(Decimal::infinity()), Decimal::from_str("+1/0"));
        assert_eq!(Ok(Decimal::neg_infinity()), Decimal::from_str("-1/0"));
        assert_eq!(Ok(Decimal::nan()), Decimal::from_str("0/0"));

        assert_eq!(
            Ok(GenericDecimal::<u8, u8>::infinity()),
            GenericDecimal::<u8, u8>::from_str("1/0")
        );
        assert_eq!(
            Ok(GenericDecimal::<u8, u8>::neg_infinity()),
            GenericDecimal::<u8, u8>::from_str("-1/0")
        );
        assert_eq!(
            Ok(GenericDecimal::<u8, u8>::nan()),
            GenericDecimal::<u8, u8>::from_str("0/0")
        );

        assert_eq!(Decimal::from("1/0"), Decimal::infinity());
    }

    #[test]
    fn from_str_infallible_conversion_rejects_component_signs_as_nan() {
        assert!(Decimal::from("1/+2").is_nan());
        assert!(Decimal::from("1.+5").is_nan());
    }

    #[test]
    fn from_str_component_sign_rejected_for_signed_storage() {
        type SignedDecimal = GenericDecimal<i16, u8>;

        assert_eq!(
            SignedDecimal::from_str("-1/2").unwrap(),
            SignedDecimal::from_fraction_with_precision(GenericFraction::new_neg(1i16, 2i16), 16)
        );
        for input in ["1/-2", "1/+2", "--1/2", "1.-2", "-32768", "1/-32768"] {
            assert_eq!(
                Err(ParseError::ParseIntError),
                SignedDecimal::from_str(input),
                "input should not place a sign inside a decimal component: {input}"
            );
        }
    }

    // TODO: more tests
}
