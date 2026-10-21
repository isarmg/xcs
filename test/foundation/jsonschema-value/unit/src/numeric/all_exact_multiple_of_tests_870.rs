#[cfg(all(test, feature = "arbitrary-precision"))]
mod exact_multiple_of_tests {
    use super::is_multiple_of_integer;
    use json_codec::{Number, Value};
    use test_case::test_case;

    fn number(raw: &str) -> Number {
        match json_codec::from_str::<Value>(raw).expect("valid JSON number") {
            Value::Number(num) => num,
            _ => unreachable!(),
        }
    }

    // Integers past `u64` still divide exactly; rounding them into `f64` first answers about a
    // different number.
    #[test_case("135107988821114880000000000000", 3.0, true; "multiple of three")]
    #[test_case("135107988821114880000000000001", 3.0, false; "one past a multiple of three")]
    #[test_case("135107988821114880000000000002", 3.0, false; "two past a multiple of three")]
    #[test_case("18446744073709551617", 2.0, false; "odd just past u64")]
    #[test_case("18446744073709551618", 2.0, true; "even just past u64")]
    #[test_case("1e30", 3.0, false; "scientific not a multiple")]
    #[test_case("1e30", 2.0, true; "scientific is a multiple")]
    fn exact_beyond_u64(value: &str, divisor: f64, expected: bool) {
        assert_eq!(is_multiple_of_integer(&number(value), divisor), expected);
    }
}
