use json_codec::{Map, Value};
use num_cmp::NumCmp;

macro_rules! num_cmp {
    ($left:expr, $right:expr) => {
        if let Some(b) = $right.as_u64() {
            NumCmp::num_eq($left, b)
        } else if let Some(b) = $right.as_i64() {
            NumCmp::num_eq($left, b)
        } else {
            #[cfg(feature = "arbitrary-precision")]
            {
                use crate::value::numeric::bignum;
                use fraction::BigFraction;

                let left_frac = BigFraction::from($left);

                // Check BigInt/BigFraction BEFORE f64 to avoid precision loss
                if let Some(right_bigint) = bignum::try_parse_bigint($right) {
                    let right_frac = BigFraction::from(right_bigint);
                    left_frac == right_frac
                } else if let Some(right_frac) = bignum::try_parse_bigfraction($right) {
                    left_frac == right_frac
                } else if let Some(b) = $right.as_f64() {
                    // Fallback to f64 for scientific notation or other cases
                    left_frac == BigFraction::from(b)
                } else {
                    // Can't parse right - not equal
                    false
                }
            }
            #[cfg(not(feature = "arbitrary-precision"))]
            {
                if let Some(b) = $right.as_f64() {
                    NumCmp::num_eq($left, b)
                } else {
                    unreachable!("Numbers always fit in u64/i64/f64 without arbitrary-precision")
                }
            }
        }
    };
}

/// Compare two JSON numbers for equality with arbitrary precision support
#[inline]
#[doc(hidden)]
#[must_use]
pub fn equal_numbers<L: crate::value::JsonNumber>(left: &L, right: &json_codec::Number) -> bool {
    #[cfg(feature = "arbitrary-precision")]
    {
        use crate::value::numeric::bignum;
        use fraction::BigFraction;

        // Check BigInt/BigFraction first to avoid precision loss from f64 conversion
        if let Some(left_bigint) = bignum::try_parse_bigint(&left.to_number()) {
            if let Some(right_bigint) = bignum::try_parse_bigint(right) {
                left_bigint == right_bigint
            } else if let Some(b) = right.as_u64() {
                left_bigint == ::numeric::bigint::BigInt::from(b)
            } else if let Some(b) = right.as_i64() {
                left_bigint == ::numeric::bigint::BigInt::from(b)
            } else if let Some(right_frac) = bignum::try_parse_bigfraction(right) {
                BigFraction::from(left_bigint) == right_frac
            } else if let Some(b) = right.as_f64() {
                BigFraction::from(left_bigint) == BigFraction::from(b)
            } else {
                unreachable!("Right is not parseable as any numeric type - should not happen for valid JSON numbers")
            }
        } else if let Some(left_frac) = bignum::try_parse_bigfraction(&left.to_number()) {
            if let Some(right_frac) = bignum::try_parse_bigfraction(right) {
                left_frac == right_frac
            } else if let Some(right_bigint) = bignum::try_parse_bigint(right) {
                left_frac == BigFraction::from(right_bigint)
            } else if let Some(b) = right.as_u64() {
                left_frac == BigFraction::from(b)
            } else if let Some(b) = right.as_i64() {
                left_frac == BigFraction::from(b)
            } else if let Some(b) = right.as_f64() {
                left_frac == BigFraction::from(b)
            } else {
                unreachable!("Right is not parseable as any numeric type - should not happen for valid JSON numbers")
            }
        } else if let Some(a) = left.as_u64() {
            num_cmp!(a, right)
        } else if let Some(a) = left.as_i64() {
            num_cmp!(a, right)
        } else if let Some(a) = left.as_f64() {
            num_cmp!(a, right)
        } else {
            // Left is a number in scientific notation that doesn't fit in f64
            // (e.g., 1e309, 1e400). With arbitrary-precision, these are stored as
            // strings but can't be converted to any numeric type we support.
            // Return false as we can't reliably compare them.
            false
        }
    }
    #[cfg(not(feature = "arbitrary-precision"))]
    {
        if let Some(a) = left.as_u64() {
            num_cmp!(a, right)
        } else if let Some(a) = left.as_i64() {
            num_cmp!(a, right)
        } else if let Some(a) = left.as_f64() {
            num_cmp!(a, right)
        } else {
            // Past `f64`, with no exact arithmetic to place it against the literal.
            false
        }
    }
}

/// Tests for two JSON values to be equal using the JSON Schema semantic.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::String(left), Value::String(right)) => left == right,
        (Value::Bool(left), Value::Bool(right)) => left == right,
        (Value::Null, Value::Null) => true,
        (Value::Number(left), Value::Number(right)) => equal_numbers(left, right),
        (Value::Array(left), Value::Array(right)) => equal_arrays(left, right),
        (Value::Object(left), Value::Object(right)) => equal_objects(left, right),
        (_, _) => false,
    }
}

#[inline]
#[must_use]
pub fn equal_arrays(left: &[Value], right: &[Value]) -> bool {
    left.len() == right.len() && {
        let mut idx = 0_usize;
        while idx < left.len() {
            if !equal(&left[idx], &right[idx]) {
                return false;
            }
            idx += 1;
        }
        true
    }
}

#[inline]
#[must_use]
pub fn equal_objects(left: &Map<String, Value>, right: &Map<String, Value>) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|((ka, va), (kb, vb))| ka == kb && equal(va, vb))
}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../../test/foundation/schema-validation/upstream/jsonschema-value/unit/src/cmp/tests_152.rs"
));
