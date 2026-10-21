#[cfg(test)]
mod tests {
    use super::{decimal_parts, divides_exactly};
    use test_case::test_case;

    #[test_case(0.1, Some((1, 1)); "leading zero")]
    #[test_case(2.675, Some((2675, 3)); "three decimals")]
    #[test_case(-0.25, Some((-25, 2)); "negative")]
    // `zmij` always writes a fractional part, so an integral value scales by ten.
    #[test_case(7.0, Some((70, 1)); "integral")]
    #[test_case(1e-7, Some((1, 7)); "negative exponent")]
    #[test_case(1e300, Some((1, -300)); "positive exponent")]
    #[test_case(f64::NAN, None; "not a number")]
    #[test_case(f64::INFINITY, None; "infinite")]
    fn decimal_parts_reads_the_printed_decimal(value: f64, expected: Option<(i128, i32)>) {
        assert_eq!(decimal_parts(value), expected);
    }

    // `BigFraction::from(f64)` rounds these into non-multiples; the decimal reading does not.
    #[test_case(1_070_468.14, 0.01, true; "large amount of cents")]
    #[test_case(1_070_468.13, 0.01, true; "another large amount of cents")]
    #[test_case(1_070_468.145, 0.01, false; "large amount of half cents")]
    #[test_case(19.99, 0.01, true; "small amount of cents")]
    #[test_case(0.0075, 0.0001, true; "fourth decimal place")]
    #[test_case(5.35, 2.675, true; "fractional divisor")]
    #[test_case(505_661.899_999_999_97, 0.1, false; "seventeen significant digits")]
    fn divides_exactly_answers(value: f64, multiple: f64, expected: bool) {
        assert_eq!(divides_exactly(value, multiple), Some(expected));
    }

    #[test_case(1e300; "too large to scale into i128")]
    #[test_case(1e-300; "too small to scale into i128")]
    fn divides_exactly_defers_out_of_range(value: f64) {
        assert_eq!(divides_exactly(value, 0.01), None);
    }
}
