#[cfg(test)]
mod tests {
    use super::*;
    use fraction::GenericFraction;
    use juniper::{FromInputValue, InputValue, ToInputValue};

    type D = GenericDecimal<u64, u8>;
    type F = GenericFraction<u64>;
    fn get_tests() -> Vec<(&'static str, D)> {
        vec![
            ("NaN", D::nan()),
            ("-inf", D::neg_infinity()),
            ("inf", D::infinity()),
            ("0", D::from_fraction(F::new(0u64, 1u64))),
            ("2", D::from_fraction(F::new(2u64, 1u64))),
            ("-2", D::from_fraction(F::new_neg(2u64, 1u64))),
            ("0.5", D::from_fraction(F::new(1u64, 2u64))),
            ("-0.5", D::from_fraction(F::new_neg(1u64, 2u64))),
            ("0.625", D::from_fraction(F::new(5u64, 8u64))),
            (
                "246.5856420264",
                D::from_fraction(F::new(308232052533u64, 1250000000u64)),
            ),
        ]
    }

    #[test]
    fn to_input_value() {
        for (s, v) in get_tests() {
            let value = <D as ToInputValue>::to_input_value(&v);
            let str_value = value.as_scalar_value::<String>();

            assert!(str_value.is_some());
            assert_eq!(s, str_value.unwrap());
        }
    }

    #[test]
    fn from_input_value() {
        for (s, v) in get_tests() {
            let value = <D as FromInputValue>::from_input_value(&InputValue::scalar(s.to_owned()));

            assert_eq!(value, Some(v));
            assert_eq!(value.unwrap().get_precision(), v.get_precision())
        }
    }

    #[test]
    fn from_input_value_component_sign_rejected_for_signed_storage() {
        type SignedDecimal = GenericDecimal<i16, u8>;

        assert!(
            <SignedDecimal as FromInputValue>::from_input_value(&InputValue::scalar("-1/2"))
                .is_some()
        );
        for input in ["1/-2", "1/+2", "--1/2", "1/-128"] {
            assert_eq!(
                <SignedDecimal as FromInputValue>::from_input_value(&InputValue::scalar(input)),
                None,
                "input should not place a sign inside a decimal Juniper component: {input}"
            );
        }
    }
}
