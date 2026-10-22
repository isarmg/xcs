// `pub` only for the `__private` re-export consumed by generated code.
#![allow(clippy::must_use_candidate, clippy::missing_errors_doc)]

use crate::validation::error::ValidationError;
use ahash::AHashMap;
use data_encoding::{BASE32, BASE32HEX, BASE64, BASE64URL, HEXUPPER};
use std::sync::LazyLock;

pub(crate) type ContentEncodingCheckType = fn(&str) -> bool;
pub(crate) type ContentEncodingConverterType =
    fn(&str) -> Result<Option<String>, ValidationError<'static>>;

// RFC 4648 §4: Base 64 Encoding
// https://datatracker.ietf.org/doc/html/rfc4648#section-4
pub fn is_base64(instance_string: &str) -> bool {
    BASE64.decode(instance_string.as_bytes()).is_ok()
}

pub fn from_base64(instance_string: &str) -> Result<Option<String>, ValidationError<'static>> {
    match BASE64.decode(instance_string.as_bytes()) {
        Ok(value) => Ok(Some(String::from_utf8(value)?)),
        Err(_) => Ok(None),
    }
}

// RFC 4648 §5: Base 64 Encoding with URL and Filename Safe Alphabet
// https://datatracker.ietf.org/doc/html/rfc4648#section-5
pub fn is_base64url(instance_string: &str) -> bool {
    BASE64URL.decode(instance_string.as_bytes()).is_ok()
}

pub fn from_base64url(instance_string: &str) -> Result<Option<String>, ValidationError<'static>> {
    match BASE64URL.decode(instance_string.as_bytes()) {
        Ok(value) => Ok(Some(String::from_utf8(value)?)),
        Err(_) => Ok(None),
    }
}

// RFC 4648 §6: Base 32 Encoding
// https://datatracker.ietf.org/doc/html/rfc4648#section-6
pub fn is_base32(instance_string: &str) -> bool {
    BASE32.decode(instance_string.as_bytes()).is_ok()
}

pub fn from_base32(instance_string: &str) -> Result<Option<String>, ValidationError<'static>> {
    match BASE32.decode(instance_string.as_bytes()) {
        Ok(value) => Ok(Some(String::from_utf8(value)?)),
        Err(_) => Ok(None),
    }
}

// RFC 4648 §7: Base 32 Encoding with Extended Hex Alphabet
// https://datatracker.ietf.org/doc/html/rfc4648#section-7
pub fn is_base32hex(instance_string: &str) -> bool {
    BASE32HEX.decode(instance_string.as_bytes()).is_ok()
}

pub fn from_base32hex(instance_string: &str) -> Result<Option<String>, ValidationError<'static>> {
    match BASE32HEX.decode(instance_string.as_bytes()) {
        Ok(value) => Ok(Some(String::from_utf8(value)?)),
        Err(_) => Ok(None),
    }
}

// RFC 4648 §8: Base 16 Encoding
// https://datatracker.ietf.org/doc/html/rfc4648#section-8
pub fn is_base16(instance_string: &str) -> bool {
    HEXUPPER.decode(instance_string.as_bytes()).is_ok()
        || HEXUPPER
            .decode(instance_string.to_uppercase().as_bytes())
            .is_ok()
}

pub fn from_base16(instance_string: &str) -> Result<Option<String>, ValidationError<'static>> {
    // Base16 is case-insensitive per RFC 4648
    let result = HEXUPPER
        .decode(instance_string.as_bytes())
        .or_else(|_| HEXUPPER.decode(instance_string.to_uppercase().as_bytes()));
    match result {
        Ok(value) => Ok(Some(String::from_utf8(value)?)),
        Err(_) => Ok(None),
    }
}

// Supported in JSON Schema Draft 6, 7, 2019-09, and 2020-12
// Per JSON Schema Validation spec §8.3, encoding values are defined in:
// - RFC 4648 (base16, base32, base32hex, base64, base64url)
// - RFC 2045 §6.7-6.8 (quoted-printable, 7bit, 8bit, binary)
// We implement the RFC 4648 encodings as they are transformation encodings.
pub(crate) static DEFAULT_CONTENT_ENCODING_CHECKS_AND_CONVERTERS: LazyLock<
    AHashMap<&'static str, (ContentEncodingCheckType, ContentEncodingConverterType)>,
> = LazyLock::new(|| {
    let mut map: AHashMap<&'static str, (ContentEncodingCheckType, ContentEncodingConverterType)> =
        AHashMap::with_capacity(5);
    map.insert("base64", (is_base64, from_base64));
    map.insert("base64url", (is_base64url, from_base64url));
    map.insert("base32", (is_base32, from_base32));
    map.insert("base32hex", (is_base32hex, from_base32hex));
    map.insert("base16", (is_base16, from_base16));
    map
});

#[cfg(test)]
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../test/foundation/schema-validation/upstream/jsonschema/unit/src/content_encoding/tests_103.rs"
));
