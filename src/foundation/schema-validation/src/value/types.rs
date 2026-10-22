//! JSON type representations for schema validation.
//!
//! Provides [`JsonType`] for individual types and [`JsonTypeSet`] for efficient
//! bitset-based type checking in validation hot paths.

use core::fmt;
use std::str::FromStr;

use json_codec::Value;

use crate::value::{Json, Node};

/// Represents a JSON value type.
///
/// Discriminant order is `Null < Boolean < Integer < Number < String < Array < Object`
/// (primitives before compounds, integers before numbers); [`JsonTypeSet`] iterates in this order.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(u8)]
pub enum JsonType {
    Null = 1 << 0,
    Boolean = 1 << 1,
    Integer = 1 << 2,
    Number = 1 << 3,
    String = 1 << 4,
    Array = 1 << 5,
    Object = 1 << 6,
}

impl fmt::Display for JsonType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl JsonType {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            JsonType::Array => "array",
            JsonType::Boolean => "boolean",
            JsonType::Integer => "integer",
            JsonType::Null => "null",
            JsonType::Number => "number",
            JsonType::Object => "object",
            JsonType::String => "string",
        }
    }

    pub(crate) fn from_repr(repr: u8) -> Self {
        match repr {
            1 => JsonType::Null,
            2 => JsonType::Boolean,
            4 => JsonType::Integer,
            8 => JsonType::Number,
            16 => JsonType::String,
            32 => JsonType::Array,
            64 => JsonType::Object,
            _ => panic!("Invalid JsonType representation: {repr}"),
        }
    }
}

impl From<&Value> for JsonType {
    fn from(instance: &Value) -> Self {
        match instance {
            Value::Null => JsonType::Null,
            Value::Bool(_) => JsonType::Boolean,
            Value::Number(_) => JsonType::Number,
            Value::String(_) => JsonType::String,
            Value::Array(_) => JsonType::Array,
            Value::Object(_) => JsonType::Object,
        }
    }
}

impl FromStr for JsonType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "array" => Ok(JsonType::Array),
            "boolean" => Ok(JsonType::Boolean),
            "integer" => Ok(JsonType::Integer),
            "null" => Ok(JsonType::Null),
            "number" => Ok(JsonType::Number),
            "object" => Ok(JsonType::Object),
            "string" => Ok(JsonType::String),
            _ => Err(()),
        }
    }
}

/// A set of JSON types.
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct JsonTypeSet(u8);

impl Default for JsonTypeSet {
    fn default() -> Self {
        Self::empty()
    }
}

impl From<JsonType> for JsonTypeSet {
    #[inline]
    fn from(ty: JsonType) -> Self {
        Self(ty as u8)
    }
}

impl std::ops::BitOr for JsonType {
    type Output = JsonTypeSet;
    #[inline]
    fn bitor(self, rhs: JsonType) -> JsonTypeSet {
        JsonTypeSet::from(self).insert(rhs)
    }
}

impl std::ops::BitOr<JsonType> for JsonTypeSet {
    type Output = JsonTypeSet;
    #[inline]
    fn bitor(self, rhs: JsonType) -> JsonTypeSet {
        self.insert(rhs)
    }
}

impl JsonTypeSet {
    /// Create an empty set of types.
    #[inline]
    #[must_use]
    pub const fn empty() -> Self {
        Self(0)
    }
    /// Create a set with all possible JSON types.
    #[inline]
    #[must_use]
    pub const fn all() -> Self {
        JsonTypeSet::empty()
            .insert(JsonType::Null)
            .insert(JsonType::Boolean)
            .insert(JsonType::Integer)
            .insert(JsonType::Number)
            .insert(JsonType::String)
            .insert(JsonType::Array)
            .insert(JsonType::Object)
    }
    /// Add a type to this set and return the modified set.
    #[inline]
    #[must_use]
    pub const fn insert(mut self, ty: JsonType) -> Self {
        self.0 |= ty as u8;
        self
    }
    /// Remove a type from this set and return the modified set.
    #[inline]
    #[must_use]
    pub const fn remove(mut self, ty: JsonType) -> Self {
        self.0 &= !(ty as u8);
        self
    }
    /// Types in both sets.
    #[inline]
    #[must_use]
    pub const fn intersect(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
    /// Types in either set.
    #[inline]
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    /// Return the number of types in this set.
    #[inline]
    #[must_use]
    pub const fn len(self) -> usize {
        self.0.count_ones() as usize
    }
    /// Return `true` if the set contains no types.
    #[inline]
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }
    /// Check if this set includes the specified type.
    #[inline]
    #[must_use]
    pub fn contains(self, ty: JsonType) -> bool {
        self.0 & ty as u8 != 0
    }
    /// Whether a JSON value's type is allowed by this set.
    #[must_use]
    pub fn contains_value_type<F: Json>(self, value: &F::Node<'_>) -> bool {
        match value.json_type() {
            JsonType::Number => match value.as_number() {
                // Integers satisfy both `integer` and `number`; non-integers only `number`.
                Some(n) if crate::value::JsonNumber::is_integer(&n) => {
                    self.contains(JsonType::Integer) || self.contains(JsonType::Number)
                }
                Some(_) => self.contains(JsonType::Number),
                None => false,
            },
            other => self.contains(other),
        }
    }
    /// Get an iterator over the types in this set.
    #[inline]
    #[must_use]
    pub fn iter(&self) -> JsonTypeSetIterator {
        JsonTypeSetIterator { set: *self }
    }
}

/// Whether `n` holds an integer value per drafts 6+ (floats with a zero fractional part count as integers).
#[must_use]
pub fn number_is_integer(n: &json_codec::Number) -> bool {
    #[cfg(feature = "arbitrary-precision")]
    {
        use crate::value::numeric::bignum;
        use ::numeric::traits::One;

        if n.is_i64() || n.is_u64() {
            return true;
        }
        // With no exponent to shift the point the fractional digits settle it, which skips the
        // bignum parses below.
        let text = json_codec::Number::as_str(n);
        if let Some((integer, fraction)) = text.split_once('.') {
            if !fraction.contains(['e', 'E']) {
                return fraction.bytes().all(|byte| byte == b'0')
                    && integer
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || byte == b'-');
            }
        }

        // Important: check BigFraction BEFORE as_f64() to avoid precision loss.
        if bignum::try_parse_bigint(n).is_some() {
            true
        } else if let Some(bigfrac) = bignum::try_parse_bigfraction(n) {
            bigfrac.denom().is_none_or(One::is_one)
        } else if let Some(f) = n.as_f64() {
            f.fract() == 0.
        } else {
            // Numbers that overflow to infinity (as_f64() returns None).
            false
        }
    }
    #[cfg(not(feature = "arbitrary-precision"))]
    {
        if n.is_i64() || n.is_u64() {
            true
        } else if let Some(f) = n.as_f64() {
            f.fract() == 0.
        } else {
            unreachable!("Numbers always fit in u64/i64/f64 without arbitrary-precision")
        }
    }
}

impl IntoIterator for &JsonTypeSet {
    type Item = JsonType;
    type IntoIter = JsonTypeSetIterator;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl IntoIterator for JsonTypeSet {
    type Item = JsonType;
    type IntoIter = JsonTypeSetIterator;

    fn into_iter(self) -> Self::IntoIter {
        JsonTypeSetIterator { set: self }
    }
}

impl fmt::Debug for JsonTypeSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(")?;

        let mut iter = self.iter();

        if let Some(ty) = iter.next() {
            write!(f, "{ty}")?;
        }

        for ty in iter {
            write!(f, ", {ty}")?;
        }

        write!(f, ")")
    }
}

/// Iterator for traversing the types in a `JsonTypeSet`.
#[derive(Debug)]
pub struct JsonTypeSetIterator {
    set: JsonTypeSet,
}

impl Iterator for JsonTypeSetIterator {
    type Item = JsonType;

    fn next(&mut self) -> Option<Self::Item> {
        if self.set.0 == 0 {
            None
        } else {
            // Find the least significant bit that is set
            let lsb = self.set.0 & self.set.0.wrapping_neg();

            // Clear the least significant bit
            self.set.0 &= self.set.0 - 1;

            Some(JsonType::from_repr(lsb))
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let count = self.set.0.count_ones() as usize;
        (count, Some(count))
    }
}

impl ExactSizeIterator for JsonTypeSetIterator {}

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/schema-validation/upstream/jsonschema-value/unit/src/types/tests_325.rs"
));
