#[cfg(test)]
mod tests {
    use super::*;
    use juniper::{FromInputValue, InputValue, ToInputValue};

    #[cfg(feature = "with-bigint")]
    use crate::BigFraction;

    type F = GenericFraction<u8>;
    fn get_tests() -> Vec<(&'static str, F)> {
        vec![
            ("NaN", F::nan()),
            ("-inf", F::neg_infinity()),
            ("+inf", F::infinity()),
            ("+0", F::new(0u8, 1u8)),
            ("-0", F::new_neg(0u8, 1u8)),
            ("+2", F::new(2u8, 1u8)),
            ("-2", F::new_neg(2u8, 1u8)),
            ("+1/2", F::new(1u8, 2u8)),
            ("-1/2", F::new_neg(1u8, 2u8)),
            ("+5/6", F::new(10u8, 12u8)),
            ("-1/255", F::new_neg(1u8, 255u8)),
        ]
    }

    #[cfg(feature = "with-bigint")]
    fn get_big_tests() -> Vec<(&'static str, BigFraction)> {
        use crate::Num;
        vec![
            ("NaN", BigFraction::nan()),
            ("-inf", BigFraction::neg_infinity()),
            ("+inf", BigFraction::infinity()),
            ("+0", BigFraction::new(0u8, 1u8)),
            ("-0", BigFraction::new_neg(0u8, 1u8)),
            ("+2", BigFraction::new(2u8, 1u8)),
            ("-2", BigFraction::new_neg(2u8, 1u8)),
            ("+1/2", BigFraction::new(1u8, 2u8)),
            ("-1/2", BigFraction::new_neg(1u8, 2u8)),
            ("+5/6", BigFraction::new(10u8, 12u8)),
            ("-1/255", BigFraction::new_neg(1u8, 255u8)),
            (
                "+42090291092428642826240949012091044820/42090291092428642826240949012091044821",
                BigFraction::from_str_radix(
                    "42090291092428642826240949012091044820/42090291092428642826240949012091044821",
                    10,
                )
                .unwrap(),
            ),
        ]
    }

    #[test]
    fn to_input_value() {
        for (s, v) in get_tests() {
            let value = <F as ToInputValue>::to_input_value(&v);
            let str_value = value.as_scalar_value::<String>();

            assert!(str_value.is_some());
            assert_eq!(s, str_value.unwrap());
        }

        #[cfg(feature = "with-bigint")]
        {
            for (s, v) in get_big_tests() {
                let value = <BigFraction as ToInputValue>::to_input_value(&v);
                let str_value = value.as_string_value();

                assert!(str_value.is_some());
                assert_eq!(s, str_value.unwrap());
            }
        }
    }

    #[test]
    fn from_input_value() {
        for (s, v) in get_tests() {
            let value = <F as FromInputValue>::from_input_value(&InputValue::scalar(s.to_owned()));
            assert_eq!(value, Some(v));
        }

        #[cfg(feature = "with-bigint")]
        {
            for (s, v) in get_big_tests() {
                let value =
                    <BigFraction as FromInputValue>::from_input_value(&InputValue::scalar(s));
                assert_eq!(value, Some(v));
            }
        }
    }

    #[test]
    fn from_input_value_component_sign_rejected_for_signed_storage() {
        type SignedFrac = GenericFraction<i8>;

        for (input, expected) in [
            ("+1/2", SignedFrac::new(1, 2)),
            ("-1/2", SignedFrac::new_neg(1, 2)),
        ] {
            let value = <SignedFrac as FromInputValue>::from_input_value(&InputValue::scalar(
                input.to_owned(),
            ));
            assert_eq!(value, Some(expected));
        }

        assert_eq!(
            <SignedFrac as FromInputValue>::from_input_value(&InputValue::scalar("1/2")),
            None
        );

        for input in ["+1/-2", "-1/+2", "+-1/2", "-1/-128"] {
            assert_eq!(
                <SignedFrac as FromInputValue>::from_input_value(&InputValue::scalar(input)),
                None,
                "input should not place a sign inside a Juniper component: {input}"
            );
        }
    }
}
