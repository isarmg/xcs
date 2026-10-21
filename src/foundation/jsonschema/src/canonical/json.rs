#[cfg(feature = "arbitrary-precision")]
use std::borrow::Cow;
use std::{cell::RefCell, fmt, io, mem};

#[cfg(feature = "arbitrary-precision")]
use serde::ser::SerializeStruct;
use serde::{
    ser::{self, Serialize, SerializeMap, SerializeSeq},
    Serializer,
};
use serde_json::{
    ser::{CompactFormatter, Formatter},
    Number, Value,
};

pub(crate) const I64_UPPER_EXCLUSIVE_F64: f64 = 9_223_372_036_854_775_808.0;
const I64_LOWER_INCLUSIVE_F64: f64 = -9_223_372_036_854_775_808.0;
pub(crate) const U64_UPPER_EXCLUSIVE_F64: f64 = 18_446_744_073_709_551_616.0;
const RECURSION_LIMIT: u16 = 255;
const MAX_SCRATCH_POOL_SIZE: usize = 8;
const MAX_SCRATCH_CAPACITY: usize = 16_384;
#[cfg(feature = "arbitrary-precision")]
const SERDE_JSON_NUMBER_TOKEN: &str = "$serde_json::private::Number";
#[cfg(feature = "arbitrary-precision")]
const MAX_EXPANDED_INTEGER_DIGITS: usize = 1 << 20;

/// An integer-valued `f64` within the `u64` range as `u64`, else `None`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "guarded by the `0.0..U64_UPPER_EXCLUSIVE_F64` range and zero fractional part"
)]
pub(crate) fn integer_valued_u64(value: f64) -> Option<u64> {
    (value.fract() == 0.0 && (0.0..U64_UPPER_EXCLUSIVE_F64).contains(&value))
        .then_some(value as u64)
}

/// An integer-valued `f64` within the `i64` range as `i64`, else `None`.
#[expect(
    clippy::cast_possible_truncation,
    reason = "guarded by the `I64_LOWER_INCLUSIVE_F64..I64_UPPER_EXCLUSIVE_F64` range and zero fractional part"
)]
pub(crate) fn integer_valued_i64(value: f64) -> Option<i64> {
    (value.fract() == 0.0 && (I64_LOWER_INCLUSIVE_F64..I64_UPPER_EXCLUSIVE_F64).contains(&value))
        .then_some(value as i64)
}

/// Error returned by [`to_string`].
#[derive(Debug)]
pub struct Error {
    inner: serde_json::Error,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.inner)
    }
}

impl From<serde_json::Error> for Error {
    fn from(inner: serde_json::Error) -> Self {
        Error { inner }
    }
}

/// Serialize a JSON value into a deterministic canonical JSON string, so equivalent values share one text.
///
/// # Rules
///
/// - Object keys are emitted in lexicographic order.
/// - Integer-valued floats are emitted as integers (`1.0` becomes `1`).
/// - With `arbitrary-precision`, one normal form per value: plain integer/decimal while the expansion fits the digit
///   cap, normalized scientific (`d[.rest]e{E}`) past it. The IR relies on this for value text equality and type scanning.
/// - Output is always compact (no extra whitespace).
///
/// # Examples
///
/// ```rust
/// use serde_json::json;
///
/// let schema = json!({"b": 1, "a": {"y": 2, "x": 3}});
/// let canonical = jsonschema::canonical::json::to_string(&schema).unwrap();
/// assert_eq!(canonical, r#"{"a":{"x":3,"y":2},"b":1}"#);
/// ```
///
/// # Errors
///
/// Returns an error if serialization fails, e.g. when the input exceeds the recursion limit.
#[allow(
    clippy::missing_panics_doc,
    reason = "the canonical serializer emits valid UTF-8 by construction; the expect is unreachable"
)]
pub fn to_string(value: &Value) -> Result<String, Error> {
    let mut output = Vec::with_capacity(initial_output_capacity(value));
    let formatter = CanonicalFormatter {
        default: CompactFormatter,
    };
    let scratch_pool = RefCell::new(Vec::new());
    let mut serializer = serde_json::Serializer::with_formatter(&mut output, formatter);
    CanonicalValue::new(value, 0, &scratch_pool)
        .serialize(&mut serializer)
        .map_err(Error::from)?;
    Ok(String::from_utf8(output).expect("canonical serializer emits valid UTF-8"))
}

#[inline]
fn initial_output_capacity(value: &Value) -> usize {
    const MIN_CAPACITY: usize = 16;
    const MAX_PREALLOC: usize = 1 << 20; // 1 MiB

    let estimated = match value {
        Value::Object(map) => map.len().saturating_mul(24).saturating_add(2),
        Value::Array(items) => items.len().saturating_mul(12).saturating_add(2),
        Value::String(s) => s.len().saturating_add(2),
        Value::Number(_) => 32,
        Value::Bool(_) => 8,
        Value::Null => 4,
    };

    estimated.clamp(MIN_CAPACITY, MAX_PREALLOC)
}

#[derive(Default)]
struct CanonicalFormatter {
    default: CompactFormatter,
}

/// A formatter that emits integer-valued floats as integers.
impl Formatter for CanonicalFormatter {
    #[inline]
    fn write_f64<W: io::Write + ?Sized>(&mut self, writer: &mut W, value: f64) -> io::Result<()> {
        if let Some(integer) = integer_valued_u64(value) {
            return self.default.write_u64(writer, integer);
        }
        if let Some(integer) = integer_valued_i64(value) {
            return self.default.write_i64(writer, integer);
        }
        if value.fract() == 0.0 {
            return writer.write_all(format!("{value:.0}").as_bytes());
        }
        self.default.write_f64(writer, value)
    }
}

struct CanonicalValue<'value> {
    value: &'value Value,
    recursion_depth: u16,
    scratch_pool: &'value RefCell<Vec<Vec<ObjectEntry<'value>>>>,
}

struct ObjectEntry<'value> {
    key: &'value str,
    value: &'value Value,
}

struct ObjectEntryScratch<'value, 'pool> {
    entries: Vec<ObjectEntry<'value>>,
    pool: &'pool RefCell<Vec<Vec<ObjectEntry<'value>>>>,
}

impl<'value, 'pool> ObjectEntryScratch<'value, 'pool> {
    fn with_capacity(pool: &'pool RefCell<Vec<Vec<ObjectEntry<'value>>>>, capacity: usize) -> Self {
        let mut entries = pool.borrow_mut().pop().unwrap_or_default();
        entries.clear();
        if entries.capacity() < capacity {
            entries.reserve(capacity - entries.capacity());
        }
        Self { entries, pool }
    }

    #[inline]
    fn entries_mut(&mut self) -> &mut Vec<ObjectEntry<'value>> {
        &mut self.entries
    }

    #[inline]
    fn entries(&self) -> &[ObjectEntry<'value>] {
        &self.entries
    }
}

impl Drop for ObjectEntryScratch<'_, '_> {
    fn drop(&mut self) {
        self.entries.clear();
        if self.entries.capacity() > MAX_SCRATCH_CAPACITY {
            return;
        }
        let mut pool = self.pool.borrow_mut();
        if pool.len() < MAX_SCRATCH_POOL_SIZE {
            pool.push(mem::take(&mut self.entries));
        }
    }
}

impl<'value> CanonicalValue<'value> {
    #[inline]
    const fn new(
        value: &'value Value,
        recursion_depth: u16,
        scratch_pool: &'value RefCell<Vec<Vec<ObjectEntry<'value>>>>,
    ) -> Self {
        CanonicalValue {
            value,
            recursion_depth,
            scratch_pool,
        }
    }
}

/// Emits already-canonical number text verbatim, without parsing it into a [`Number`].
#[cfg(feature = "arbitrary-precision")]
#[doc(hidden)]
pub struct BorrowedNumber<'a>(pub &'a str);

#[cfg(feature = "arbitrary-precision")]
impl Serialize for BorrowedNumber<'_> {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serialized = serializer.serialize_struct(SERDE_JSON_NUMBER_TOKEN, 1)?;
        serialized.serialize_field(SERDE_JSON_NUMBER_TOKEN, self.0)?;
        serialized.end()
    }
}

#[cfg(feature = "arbitrary-precision")]
fn push_digits(output: &mut String, digits: &[u8]) {
    output.push_str(std::str::from_utf8(digits).expect("ASCII digits"));
}

/// Canonical text for a valid JSON-number token, borrowing the input when it is already canonical.
/// `None` only when the exponent magnitude overflows `usize` (32-bit targets), leaving the raw text.
#[cfg(feature = "arbitrary-precision")]
#[doc(hidden)]
#[inline]
#[must_use]
pub fn canonical_number(raw: &str) -> Option<Cow<'_, str>> {
    // `raw` is a valid JSON-number token; the scan assumes that shape.
    let bytes = raw.as_bytes();

    let mut idx = 0;
    let negative = bytes[idx] == b'-';
    if negative {
        idx += 1;
    }

    let integer_start = idx;
    if bytes[idx] == b'0' {
        idx += 1;
    } else {
        while idx < bytes.len() && bytes[idx].is_ascii_digit() {
            idx += 1;
        }
    }
    let integer_end = idx;

    let mut fraction_start = idx;
    let mut fraction_end = idx;
    if idx < bytes.len() && bytes[idx] == b'.' {
        idx += 1;
        fraction_start = idx;
        while idx < bytes.len() && bytes[idx].is_ascii_digit() {
            idx += 1;
        }
        fraction_end = idx;
    }
    let has_fraction = fraction_end > fraction_start;

    let mut exponent: i64 = 0;
    let mut exponent_digits = "";
    let has_exponent = if idx < bytes.len() && matches!(bytes[idx], b'e' | b'E') {
        idx += 1;
        // Keep a leading `-` for `parse_bytes`; drop `+`.
        let sign_start = idx;
        let mut exponent_negative = false;
        if idx < bytes.len() && matches!(bytes[idx], b'+' | b'-') {
            exponent_negative = bytes[idx] == b'-';
            idx += 1;
        }
        let digits_start = idx;
        while idx < bytes.len() && bytes[idx].is_ascii_digit() {
            // Saturates: steers the branch below; scientific form recomputes exactly.
            exponent = exponent
                .saturating_mul(10)
                .saturating_add(i64::from(bytes[idx] - b'0'));
            idx += 1;
        }
        exponent_digits = if exponent_negative {
            exponent = -exponent;
            &raw[sign_start..idx]
        } else {
            &raw[digits_start..idx]
        };
        true
    } else {
        false
    };

    let integer_within_cap = integer_end - integer_start <= MAX_EXPANDED_INTEGER_DIGITS;
    let fraction_within_cap = fraction_end - fraction_start <= MAX_EXPANDED_INTEGER_DIGITS;

    if !has_fraction && !has_exponent && integer_within_cap {
        return Some(Cow::Borrowed(raw));
    }

    // In-cap plain decimal with a non-zero last digit is already canonical: nothing is left to strip,
    // so the fraction run is exactly the point offset the expansion carries, under the same cap.
    if !has_exponent && integer_within_cap && fraction_within_cap && bytes[fraction_end - 1] != b'0'
    {
        return Some(Cow::Borrowed(raw));
    }

    let integer_digits = &raw[integer_start..integer_end];
    let fraction_digits = &raw[fraction_start..fraction_end];
    let mut digits = Vec::with_capacity(integer_digits.len() + fraction_digits.len());
    digits.extend_from_slice(integer_digits.as_bytes());
    digits.extend_from_slice(fraction_digits.as_bytes());

    // Strip leading zeros so the cap check below can't split equal values across branches.
    let leading_zeros = digits.iter().take_while(|&&byte| byte == b'0').count();
    digits.drain(..leading_zeros);
    if digits.is_empty() {
        return Some(Cow::Borrowed("0"));
    }

    let parts = NumberParts {
        fraction_len: fraction_digits.len(),
        exponent_digits,
        negative,
    };

    let fraction_len = i64::try_from(parts.fraction_len).unwrap_or(i64::MAX);
    let shift = exponent.saturating_sub(fraction_len);

    let prefix_len = if shift >= 0 {
        let extra_zeros = usize::try_from(shift).ok()?;
        let expanded_len = digits.len().checked_add(extra_zeros)?;
        if expanded_len > MAX_EXPANDED_INTEGER_DIGITS {
            return canonical_scientific_number(&digits, &parts).map(Cow::Owned);
        }
        digits.resize(expanded_len, b'0');
        digits.len()
    } else {
        let drop_len = usize::try_from(shift.unsigned_abs()).ok()?;
        if drop_len <= digits.len()
            && digits[digits.len() - drop_len..]
                .iter()
                .all(|&byte| byte == b'0')
        {
            digits.len() - drop_len
        } else {
            return canonical_fractional_number(&digits, drop_len, &parts).map(Cow::Owned);
        }
    };

    // Prefix is non-empty with no leading zero.
    let prefix = &digits[..prefix_len];
    let mut output = String::with_capacity(prefix.len() + usize::from(negative));
    if negative {
        output.push('-');
    }
    push_digits(&mut output, prefix);
    Some(Cow::Owned(output))
}

/// Parsed number facets needed past the digit buffer: fraction length, the exact exponent digits (the
/// `i64` working exponent saturates; the slice keeps a leading `-`), and the sign.
#[cfg(feature = "arbitrary-precision")]
struct NumberParts<'a> {
    fraction_len: usize,
    exponent_digits: &'a str,
    negative: bool,
}

/// Plain-decimal normal form: `digits` (leading zeros pre-stripped) sit `point_offset` places right of the
/// point. Trailing zeros are stripped before the `MAX_EXPANDED_INTEGER_DIGITS` cap check so equal values never
/// straddle it; past the cap the scientific form takes over.
#[cfg(feature = "arbitrary-precision")]
fn canonical_fractional_number(
    digits: &[u8],
    point_offset: usize,
    parts: &NumberParts<'_>,
) -> Option<String> {
    let last_non_zero = digits.iter().rposition(|&byte| byte != b'0')?;
    // Stripped trailing zeros sit right of the point, so the offset stays positive.
    let point_offset = point_offset.checked_sub(digits.len() - 1 - last_non_zero)?;
    if point_offset > MAX_EXPANDED_INTEGER_DIGITS {
        return canonical_scientific_number(digits, parts);
    }
    let stripped = &digits[..=last_non_zero];
    let mut output = String::with_capacity(
        stripped
            .len()
            .max(point_offset)
            .saturating_add(2)
            .saturating_add(usize::from(parts.negative)),
    );
    if parts.negative {
        output.push('-');
    }
    if stripped.len() > point_offset {
        let integer_len = stripped.len() - point_offset;
        push_digits(&mut output, &stripped[..integer_len]);
        output.push('.');
        push_digits(&mut output, &stripped[integer_len..]);
    } else {
        output.push_str("0.");
        output.extend(std::iter::repeat_n('0', point_offset - stripped.len()));
        push_digits(&mut output, stripped);
    }
    Some(output)
}

/// Scientific normal form `d[.rest]e{E}` for values whose plain expansion exceeds `MAX_EXPANDED_INTEGER_DIGITS`.
/// `digits` carry `digits x 10^(exponent - fraction_len)`, so `E = exponent + len(digits) - fraction_len - 1`.
#[cfg(feature = "arbitrary-precision")]
fn canonical_scientific_number(digits: &[u8], parts: &NumberParts<'_>) -> Option<String> {
    use num_bigint::BigInt;

    // The plain-decimal re-dispatch has no exponent; `parse_bytes` on an empty slice gives `None`.
    let mut exponent = if parts.exponent_digits.is_empty() {
        BigInt::from(0)
    } else {
        BigInt::parse_bytes(parts.exponent_digits.as_bytes(), 10)?
    };
    exponent += BigInt::from(i64::try_from(digits.len()).ok()?);
    exponent -= BigInt::from(i64::try_from(parts.fraction_len).ok()?) + BigInt::from(1);

    let last_non_zero = digits.iter().rposition(|&byte| byte != b'0')?;
    let significand = &digits[..=last_non_zero];
    let exponent = exponent.to_string();
    let mut output = String::with_capacity(
        significand
            .len()
            .saturating_add(exponent.len())
            .saturating_add(3),
    );
    if parts.negative {
        output.push('-');
    }
    output.push(char::from(significand[0]));
    if significand.len() > 1 {
        output.push('.');
        push_digits(&mut output, &significand[1..]);
    }
    output.push('e');
    output.push_str(&exponent);
    Some(output)
}

// `Number::from_f64` formats to shortest-roundtrip text before this runs, so the double nearest
// `1e300` and exactly `1e300` collide here. The bindings convert native floats exactly and do not.
fn serialize_number<S>(number: &Number, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    #[cfg(feature = "arbitrary-precision")]
    {
        if let Some(text) = canonical_number(number.as_str()) {
            return serializer.serialize_some(&BorrowedNumber(&text));
        }
    }
    number.serialize(serializer)
}

impl Serialize for CanonicalValue<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.value {
            Value::Null => serializer.serialize_unit(),
            Value::Bool(value) => serializer.serialize_bool(*value),
            Value::Number(number) => serialize_number(number, serializer),
            Value::String(value) => serializer.serialize_str(value),
            Value::Array(items) => {
                if self.recursion_depth == RECURSION_LIMIT {
                    return Err(ser::Error::custom("Recursion limit reached"));
                }
                let mut sequence = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    sequence.serialize_element(&CanonicalValue::new(
                        item,
                        self.recursion_depth + 1,
                        self.scratch_pool,
                    ))?;
                }
                sequence.end()
            }
            Value::Object(map) => {
                if self.recursion_depth == RECURSION_LIMIT {
                    return Err(ser::Error::custom("Recursion limit reached"));
                }
                let mut output = serializer.serialize_map(Some(map.len()))?;
                // Always sort keys: downstream crates can enable `serde_json/preserve_order` transitively.
                let mut scratch = ObjectEntryScratch::with_capacity(self.scratch_pool, map.len());
                {
                    let entries = scratch.entries_mut();
                    for (key, value) in map {
                        entries.push(ObjectEntry {
                            key: key.as_str(),
                            value,
                        });
                    }
                    entries.sort_unstable_by(|left, right| {
                        left.key.as_bytes().cmp(right.key.as_bytes())
                    });
                }
                for entry in scratch.entries() {
                    output.serialize_entry(
                        entry.key,
                        &CanonicalValue::new(
                            entry.value,
                            self.recursion_depth + 1,
                            self.scratch_pool,
                        ),
                    )?;
                }
                output.end()
            }
        }
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/jsonschema/unit/src/canonical/json/tests_533.rs"));
