#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case("1.5", 2, 1; "positive fraction")]
    #[test_case("-1.5", -1, -2; "negative fraction")]
    #[test_case("2", 2, 2; "positive whole")]
    #[test_case("-2", -2, -2; "negative whole")]
    #[test_case("0.0001", 1, 0; "just above zero")]
    #[test_case("-0.5", 0, -1; "just below zero")]
    #[test_case("0", 0, 0; "zero")]
    fn rounds_toward_the_named_direction(text: &str, up: i64, down: i64) {
        let number: json_codec::Number = text.parse().expect("number");
        let expected = |value: i64| BoundInteger::from_number(&value.into()).expect("whole");
        assert_eq!(
            BoundInteger::round_from_number(&number, Round::Up),
            Some(expected(up))
        );
        assert_eq!(
            BoundInteger::round_from_number(&number, Round::Down),
            Some(expected(down))
        );
    }
}
