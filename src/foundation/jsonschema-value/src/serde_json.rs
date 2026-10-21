//! `serde_json::Value` representation: borrow-only accessors that monomorphize to direct `&Value` code.

use std::borrow::Cow;

use serde_json::{Map, Value};

use crate::{cmp, types::JsonType};

use super::{Array, Json, Node, NodeIdentity, Object};

pub struct SerdeJson;

impl Json for SerdeJson {
    type Node<'a> = &'a Value;
    type PreparedKey = String;
    type StringBuffer = Value;

    // `Map` is a `BTreeMap`, so a lookup orders keys and reaches `memcmp` on every probe.
    const KEYS_PER_LOOKUP: usize = 2;

    fn prepare_key(key: &str) -> String {
        key.to_owned()
    }

    fn with_string_node<T>(buffer: &mut Value, string: &str, f: impl FnOnce(&Value) -> T) -> T {
        // Reuses the buffer's allocation across calls instead of building a fresh `String` per name.
        if let Value::String(existing) = buffer {
            existing.clear();
            existing.push_str(string);
        } else {
            *buffer = Value::String(string.to_owned());
        }
        f(buffer)
    }
}

impl<'a> Node<'a, SerdeJson> for &'a Value {
    type Object = &'a Map<String, Value>;
    type Array = &'a [Value];
    type Number = &'a serde_json::Number;

    fn as_object(&self) -> Option<&'a Map<String, Value>> {
        match self {
            Value::Object(members) => Some(members),
            _ => None,
        }
    }

    fn as_array(&self) -> Option<&'a [Value]> {
        match self {
            Value::Array(items) => Some(items),
            _ => None,
        }
    }

    fn as_string(&self) -> Option<Cow<'a, str>> {
        match self {
            Value::String(string) => Some(Cow::Borrowed(string)),
            _ => None,
        }
    }

    fn as_number(&self) -> Option<&'a serde_json::Number> {
        match self {
            Value::Number(number) => Some(number),
            _ => None,
        }
    }

    fn as_boolean(&self) -> Option<bool> {
        match self {
            Value::Bool(boolean) => Some(*boolean),
            _ => None,
        }
    }

    fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    fn json_type(&self) -> JsonType {
        match self {
            Value::Null => JsonType::Null,
            Value::Bool(_) => JsonType::Boolean,
            Value::Number(_) => JsonType::Number,
            Value::String(_) => JsonType::String,
            Value::Array(_) => JsonType::Array,
            Value::Object(_) => JsonType::Object,
        }
    }

    fn string_length(&self) -> Option<u64> {
        match self {
            // SIMD-accelerated counting; the default `chars().count()` is measurably slower.
            Value::String(string) => Some(bytecount::num_chars(string.as_bytes()) as u64),
            _ => None,
        }
    }

    fn equals_value(&self, expected: &Value) -> bool {
        cmp::equal(self, expected)
    }

    fn to_value(&self) -> Cow<'a, Value> {
        Cow::Borrowed(self)
    }

    fn identity(&self) -> Option<NodeIdentity> {
        Some(NodeIdentity::new(std::ptr::from_ref::<Value>(self) as usize))
    }
}

pub struct SerdeMembersIter<'a>(serde_json::map::Iter<'a>);

impl<'a> Iterator for SerdeMembersIter<'a> {
    type Item = (&'a str, &'a Value);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(key, value)| (key.as_str(), value))
    }
}

impl<'a> Object<'a, SerdeJson> for &'a Map<String, Value> {
    type Node = &'a Value;
    type MemberName = &'a str;
    type MembersIter = SerdeMembersIter<'a>;

    fn len(&self) -> usize {
        Map::len(self)
    }

    fn get(&self, key: &String) -> Option<&'a Value> {
        (*self).get(key.as_str())
    }

    fn members(&self) -> SerdeMembersIter<'a> {
        SerdeMembersIter((*self).iter())
    }
}

impl<'a> Array<'a, SerdeJson> for &'a [Value] {
    type Node = &'a Value;
    type ElementsIter = std::slice::Iter<'a, Value>;

    fn len(&self) -> usize {
        <[Value]>::len(self)
    }

    fn elements(&self) -> std::slice::Iter<'a, Value> {
        (*self).iter()
    }

    fn is_unique(&self) -> bool {
        crate::unique::is_unique(self)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/jsonschema-value/unit/src/serde_json/tests_158.rs"));
