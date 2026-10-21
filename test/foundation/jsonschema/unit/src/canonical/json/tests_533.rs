#[cfg(test)]
mod tests {
    use json_codec::{json, Map, Value};
    use test_case::test_case;

    use super::to_string;

    #[test]
    fn canonical_string_is_stable_for_equivalent_schemas() {
        let left: Value =
            json_codec::from_str(r#"{"b":1,"a":{"z":3,"x":1,"y":2},"c":[{"d":4,"b":2}]}"#).unwrap();
        let right: Value =
            json_codec::from_str(r#"{"c":[{"b":2,"d":4}],"a":{"y":2,"x":1,"z":3},"b":1}"#).unwrap();

        assert_eq!(to_string(&left).unwrap(), to_string(&right).unwrap());
    }

    #[test_case("null"; "null")]
    #[test_case("true"; "bool_true")]
    #[test_case("false"; "bool_false")]
    #[test_case(r#""hello""#; "simple")]
    #[test_case(r#""line\nbreak""#; "escaped_newline")]
    fn scalar_literals_roundtrip(raw: &str) {
        let value: Value = json_codec::from_str(raw).unwrap();
        assert_eq!(to_string(&value).unwrap(), raw);
    }

    #[test]
    fn canonical_output_is_idempotent() {
        let value: Value =
            json_codec::from_str(r#"{"z":{"b":1,"a":2},"a":[3,2,1],"f":1.0,"v":1.5}"#).unwrap();

        let first = to_string(&value).unwrap();
        let parsed: Value = json_codec::from_str(&first).unwrap();

        assert_eq!(to_string(&parsed).unwrap(), first);
    }

    fn nest_arrays(depth: usize) -> Value {
        let mut value = Value::Null;
        for _ in 0..depth {
            value = Value::Array(vec![value]);
        }
        value
    }

    fn nest_objects(depth: usize) -> Value {
        let mut value = Value::Null;
        for _ in 0..depth {
            value = Value::Object([("a".to_string(), value)].into_iter().collect());
        }
        value
    }

    #[test_case(&nest_arrays(300) ; "arrays")]
    #[test_case(&nest_objects(300) ; "objects")]
    fn recursion_limit_error_exposes_source_and_message(value: &Value) {
        let error = to_string(value).expect_err("should exceed the recursion limit");
        assert!(std::error::Error::source(&error).is_some());
        assert!(error.to_string().contains("Recursion"));
    }

    #[test]
    fn oversized_object_scratch_is_not_pooled() {
        // A scratch buffer whose capacity exceeds MAX_SCRATCH_CAPACITY is dropped instead of pooled.
        let mut map = Map::new();
        for index in 0..20_000 {
            map.insert(format!("k{index:05}"), json!(index));
        }
        let out = to_string(&Value::Object(map)).unwrap();
        assert!(out.starts_with(r#"{"k00000":0,"k00001":1,"#));
    }

    #[cfg(not(feature = "arbitrary-precision"))]
    #[test_case(&json!(1.0), "1" ; "small unsigned integral")]
    #[test_case(&json!(-5.0), "-5" ; "small signed integral")]
    #[test_case(&json!(1.5), "1.5" ; "non integral falls through")]
    fn write_f64_formats(value: &Value, expected: &str) {
        assert_eq!(to_string(value).unwrap(), expected);
    }

    #[cfg(not(feature = "arbitrary-precision"))]
    #[test_case(1e30 ; "above u64 max")]
    #[test_case(-1e30 ; "below i64 min")]
    fn large_integral_floats_have_no_exponent(value: f64) {
        let out = to_string(&json!(value)).unwrap();
        assert!(
            !out.contains('.') && !out.contains('e') && !out.contains('E'),
            "{out}"
        );
    }

    #[test]
    fn large_integer_valued_float_uses_integer_form() {
        let value: Value = json_codec::from_str("1e300").unwrap();

        #[cfg(feature = "arbitrary-precision")]
        let expected = {
            let mut output = String::with_capacity(301);
            output.push('1');
            output.push_str(&"0".repeat(300));
            output
        };
        #[cfg(not(feature = "arbitrary-precision"))]
        let expected = format!("{:.0}", 1e300_f64);

        assert_eq!(to_string(&value).unwrap(), expected);
    }

    #[cfg(feature = "arbitrary-precision")]
    fn canonical(raw: &str) -> String {
        to_string(&json_codec::from_str::<Value>(raw).unwrap()).unwrap()
    }

    // Plain expansion past the digit cap switches to scientific normal form.
    #[cfg(feature = "arbitrary-precision")]
    #[test_case("1e1048577", "1e1048577" ; "single digit significand")]
    #[test_case("12e1048576", "1.2e1048577" ; "multi digit significand")]
    #[test_case("-1e1048576", "-1e1048576" ; "negative significand")]
    // Huge negative exponent drops past the digit buffer into the fractional-then-scientific path.
    #[test_case("1e-1048577", "1e-1048577" ; "fractional past cap")]
    // All-zero fractions collapse to "0"; trailing-zero fractions expand back to a (signed) integer.
    #[test_case("0.0", "0" ; "all zero collapses")]
    #[test_case("-1.0", "-1" ; "negative integral expansion")]
    fn scientific_normal_form(raw: &str, expected: &str) {
        assert_eq!(canonical(raw), expected);
    }

    // Within-cap texts share one plain-decimal normal form.
    #[cfg(feature = "arbitrary-precision")]
    #[test_case("1e+3", "1000" ; "positive exponent expands")]
    #[test_case("12e2", "1200" ; "multi digit positive exponent expands")]
    #[test_case("100E-2", "1" ; "negative exponent integral")]
    #[test_case("1e-2", "0.01" ; "negative exponent zero padded")]
    #[test_case("3.14e-3", "0.00314" ; "fraction with exponent zero padded")]
    #[test_case("3.1400e-3", "0.00314" ; "trailing zeros stripped before padding")]
    #[test_case("1.50", "1.5" ; "trailing fraction zero stripped")]
    #[test_case("0.15e1", "1.5" ; "point shifts right")]
    #[test_case("-5e-1", "-0.5" ; "negative fraction")]
    #[test_case("0e+10", "0" ; "zero with positive exponent")]
    #[test_case("-0E-1000", "0" ; "negative zero with huge negative exponent")]
    // No exponent, already canonical, non-zero trailing digit: takes the zero-copy borrow path.
    #[test_case("1.25", "1.25" ; "fractional decimal")]
    fn plain_decimal_normal_form(raw: &str, expected: &str) {
        assert_eq!(canonical(raw), expected);
    }

    // A plain decimal past the digit cap re-dispatches to scientific form with no explicit exponent,
    // sharing one text with the equivalent scientific form. A non-zero last digit must not let the
    // already-canonical fast path keep the unexpanded text.
    #[cfg(feature = "arbitrary-precision")]
    #[test_case(&format!("0.{}10", "0".repeat(1 << 20)), "1e-1048577" ; "trailing zero")]
    #[test_case(&format!("0.{}1", "0".repeat(1 << 20)), "1e-1048577" ; "non zero last digit")]
    #[test_case(&format!("-0.{}1", "0".repeat(1 << 20)), "-1e-1048577" ; "negative non zero last digit")]
    fn oversized_plain_decimal_matches_scientific(plain: &str, scientific: &str) {
        assert_eq!(canonical(plain), scientific);
        assert_eq!(canonical(plain), canonical(scientific));
    }

    // At the cap the plain decimal is the normal form, and the scientific form expands into it.
    #[cfg(feature = "arbitrary-precision")]
    #[test]
    fn fraction_at_the_cap_stays_plain() {
        let plain = format!("0.{}1", "0".repeat((1 << 20) - 1));
        assert_eq!(canonical(&plain), plain);
        assert_eq!(canonical("1e-1048576"), plain);
    }
}
