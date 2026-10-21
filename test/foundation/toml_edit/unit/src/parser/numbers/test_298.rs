#[cfg(test)]
#[cfg(feature = "parse")]
#[cfg(feature = "display")]
mod test {
    use super::*;

    #[test]
    fn integers() {
        let cases = [
            ("+99", 99),
            ("42", 42),
            ("0", 0),
            ("-17", -17),
            ("1_000", 1_000),
            ("5_349_221", 5_349_221),
            ("1_2_3_4_5", 1_2_3_4_5),
            ("0xF", 15),
            ("0o0_755", 493),
            ("0b1_0_1", 5),
            (&i64::MIN.to_string()[..], i64::MIN),
            (&i64::MAX.to_string()[..], i64::MAX),
        ];
        for &(input, expected) in &cases {
            dbg!(input);
            let parsed = integer.parse(new_input(input));
            assert_eq!(parsed, Ok(expected), "Parsing {input:?}");
        }

        let overflow = "1000000000000000000000000000000000";
        let parsed = integer.parse(new_input(overflow));
        assert!(parsed.is_err());
    }

    #[track_caller]
    fn assert_float_eq(actual: f64, expected: f64) {
        if expected.is_nan() {
            assert!(actual.is_nan());
            assert_eq!(expected.is_sign_positive(), actual.is_sign_positive());
        } else if expected.is_infinite() {
            assert!(actual.is_infinite());
            assert_eq!(expected.is_sign_positive(), actual.is_sign_positive());
        } else {
            dbg!(expected);
            dbg!(actual);
            assert!((expected - actual).abs() < f64::EPSILON);
        }
    }

    #[test]
    fn floats() {
        let cases = [
            ("+1.0", 1.0),
            ("3.1419", 3.1419),
            ("-0.01", -0.01),
            ("5e+22", 5e+22),
            ("1e6", 1e6),
            ("-2E-2", -2E-2),
            ("6.626e-34", 6.626e-34),
            ("9_224_617.445_991_228_313", 9_224_617.445_991_227),
            ("-1.7976931348623157e+308", f64::MIN),
            ("1.7976931348623157e+308", f64::MAX),
            ("nan", f64::NAN.copysign(1.0)),
            ("+nan", f64::NAN.copysign(1.0)),
            ("-nan", f64::NAN.copysign(-1.0)),
            ("inf", f64::INFINITY),
            ("+inf", f64::INFINITY),
            ("-inf", f64::NEG_INFINITY),
            // ("1e+400", f64::INFINITY),
        ];
        for &(input, expected) in &cases {
            dbg!(input);
            let parsed = float.parse(new_input(input)).unwrap();
            assert_float_eq(parsed, expected);

            let overflow = "9e99999";
            let parsed = float.parse(new_input(overflow));
            assert!(parsed.is_err(), "{parsed:?}");
        }
    }
}
