use crate::{Error, Result};
use serde::{
    Deserialize, Serialize,
    de::{self, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Map, Number, Value};
use std::fmt;

/// Reject duplicate keys before a Value or typed DTO can erase the evidence.
pub fn strict_json(bytes: &[u8]) -> Result<Value> {
    struct Strict(Value);
    impl<'de> Deserialize<'de> for Strict {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
            struct V;
            impl<'de> Visitor<'de> for V {
                type Value = Strict;
                fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    f.write_str("integer-only JSON without duplicate fields")
                }
                fn visit_bool<E: de::Error>(self, v: bool) -> std::result::Result<Strict, E> {
                    Ok(Strict(Value::Bool(v)))
                }
                fn visit_unit<E: de::Error>(self) -> std::result::Result<Strict, E> {
                    Ok(Strict(Value::Null))
                }
                fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<Strict, E> {
                    Ok(Strict(Value::Number(v.into())))
                }
                fn visit_u64<E: de::Error>(self, v: u64) -> std::result::Result<Strict, E> {
                    Ok(Strict(Value::Number(v.into())))
                }
                fn visit_f64<E: de::Error>(self, _: f64) -> std::result::Result<Strict, E> {
                    Err(E::custom("floating point is not part of the ZXS contract"))
                }
                fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Strict, E> {
                    Ok(Strict(Value::String(v.into())))
                }
                fn visit_string<E: de::Error>(self, v: String) -> std::result::Result<Strict, E> {
                    Ok(Strict(Value::String(v)))
                }
                fn visit_seq<A: SeqAccess<'de>>(
                    self,
                    mut a: A,
                ) -> std::result::Result<Strict, A::Error> {
                    let mut v = Vec::new();
                    while let Some(e) = a.next_element::<Strict>()? {
                        v.push(e.0);
                    }
                    Ok(Strict(Value::Array(v)))
                }
                fn visit_map<A: MapAccess<'de>>(
                    self,
                    mut a: A,
                ) -> std::result::Result<Strict, A::Error> {
                    let mut v = Map::new();
                    while let Some(k) = a.next_key::<String>()? {
                        if v.contains_key(&k) {
                            return Err(de::Error::custom(format!("duplicate field: {k}")));
                        }
                        v.insert(k, a.next_value::<Strict>()?.0);
                    }
                    Ok(Strict(Value::Object(v)))
                }
            }
            d.deserialize_any(V)
        }
    }
    let mut d = serde_json::Deserializer::from_slice(bytes);
    let v = Strict::deserialize(&mut d)?;
    d.end()?;
    Ok(v.0)
}

pub fn decode_strict<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    Ok(serde_json::from_value(strict_json(bytes)?)?)
}

/// JCS for the integer-only ZXS schema. UTF-16 ordering is significant for keys.
pub fn canonical_json<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>> {
    crate::integer_json::validate(value)?;
    fn number(n: &Number) -> Result<String> {
        const MAX_SAFE_INTEGER: u64 = (1 << 53) - 1;
        let safe = n
            .as_i64()
            .is_some_and(|v| v.unsigned_abs() <= MAX_SAFE_INTEGER)
            || n.as_u64().is_some_and(|v| v <= MAX_SAFE_INTEGER);
        if !safe {
            return Err(Error::new(
                "INVALID_JSON",
                "JSON numbers must be safe integers; encode u64 fields as decimal strings",
            ));
        }
        Ok(n.to_string())
    }
    fn emit(v: &Value, out: &mut Vec<u8>) -> Result<()> {
        match v {
            Value::Null => out.extend_from_slice(b"null"),
            Value::Bool(true) => out.extend_from_slice(b"true"),
            Value::Bool(false) => out.extend_from_slice(b"false"),
            Value::String(s) => out.extend(serde_json::to_vec(s)?),
            Value::Number(n) => out.extend(number(n)?.as_bytes()),
            Value::Array(a) => {
                out.push(b'[');
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(b',');
                    }
                    emit(v, out)?;
                }
                out.push(b']');
            }
            Value::Object(o) => {
                out.push(b'{');
                let mut keys = o.keys().collect::<Vec<_>>();
                keys.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
                for (i, k) in keys.into_iter().enumerate() {
                    if i > 0 {
                        out.push(b',');
                    }
                    out.extend(serde_json::to_vec(k)?);
                    out.push(b':');
                    emit(&o[k], out)?;
                }
                out.push(b'}');
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    emit(&serde_json::to_value(value)?, &mut out)?;
    Ok(out)
}
pub fn object_hash<T: Serialize + ?Sized>(kind: &str, value: &T) -> Result<String> {
    encode_hashed_object(kind, value).map(|(_, hash)| hash)
}

/// Canonical bytes and their domain-separated digest from one encoding.
/// Returning the bytes with their hash avoids serializing immutable replicas twice.
pub fn encode_hashed_object<T: Serialize + ?Sized>(
    kind: &str,
    value: &T,
) -> Result<(Vec<u8>, String)> {
    if kind.is_empty() || !kind.bytes().all(|b| b.is_ascii_lowercase() || b == b'-') {
        return Err(Error::new("INVALID_OBJECT_TYPE", "invalid metadata domain"));
    }
    let bytes = canonical_json(value)?;
    let mut h = blake3::Hasher::new();
    h.update(format!("zxs:{kind}\0").as_bytes());
    h.update(&bytes);
    Ok((bytes, h.finalize().to_hex().to_string()))
}

/// Digest for a JSON mutation request; transport credentials never enter it.
pub fn request_digest(
    method: &str,
    route: &str,
    path_parameters: Value,
    query: Value,
    body: &Value,
) -> Result<String> {
    let mut body = body.clone();
    body.as_object_mut()
        .ok_or_else(|| Error::new("INVALID_REQUEST", "request body must be an object"))?
        .remove("request_digest");
    object_hash(
        "request",
        &serde_json::json!({"method":method.to_ascii_uppercase(),"route":route,"path_parameters":path_parameters,"query":query,"body":body}),
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChecksumManifest {
    pub chunk_bytes: u32,
    #[serde(with = "crate::u64_string")]
    pub logical_size: u64,
    pub chunks: Vec<[u8; 32]>,
}
/// Supported by checksum encoding, configuration admission and package readers.
pub const SUPPORTED_CHECKSUM_CHUNK_BYTES: [u32; 4] = [65_536, 262_144, 1_048_576, 4_194_304];

/// Fixed semantic header: magic, algorithm, chunk size, logical size and count.
pub const CHECKSUM_HEADER_BYTES: usize = 30;
const CHECKSUM_MAGIC: &[u8; 8] = b"ZXSCHKS\0";

impl ChecksumManifest {
    pub fn encode(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let capacity = self
            .chunks
            .len()
            .checked_mul(32)
            .and_then(|bytes| bytes.checked_add(CHECKSUM_HEADER_BYTES))
            .ok_or_else(|| {
                Error::new(
                    "CHECKSUM_FORMAT_INVALID",
                    "checksum manifest length overflow",
                )
            })?;
        let mut out = Vec::with_capacity(capacity);
        out.extend_from_slice(CHECKSUM_MAGIC);
        out.extend(1u16.to_le_bytes());
        out.extend(self.chunk_bytes.to_le_bytes());
        out.extend(self.logical_size.to_le_bytes());
        out.extend((self.chunks.len() as u64).to_le_bytes());
        for h in &self.chunks {
            out.extend(h);
        }
        Ok(out)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let bad = || Error::new("CHECKSUM_FORMAT_INVALID", "invalid checksum manifest");
        if bytes.len() < CHECKSUM_HEADER_BYTES
            || &bytes[..8] != CHECKSUM_MAGIC
            || bytes[8..10] != 1u16.to_le_bytes()
        {
            return Err(bad());
        }
        let chunk_bytes = u32::from_le_bytes(bytes[10..14].try_into().map_err(|_| bad())?);
        let logical_size = u64::from_le_bytes(bytes[14..22].try_into().map_err(|_| bad())?);
        let count = u64::from_le_bytes(bytes[22..30].try_into().map_err(|_| bad())?);
        let payload_len = usize::try_from(count)
            .ok()
            .and_then(|count| count.checked_mul(32))
            .and_then(|bytes| bytes.checked_add(CHECKSUM_HEADER_BYTES));
        if payload_len != Some(bytes.len()) {
            return Err(bad());
        }
        let chunks = bytes[CHECKSUM_HEADER_BYTES..].as_chunks::<32>().0.to_vec();
        let m = Self {
            chunk_bytes,
            logical_size,
            chunks,
        };
        m.validate()?;
        Ok(m)
    }
    fn validate(&self) -> Result<()> {
        if !SUPPORTED_CHECKSUM_CHUNK_BYTES.contains(&self.chunk_bytes)
            || self.chunks.len() as u64 != self.logical_size.div_ceil(u64::from(self.chunk_bytes))
        {
            return Err(Error::new(
                "CHECKSUM_FORMAT_INVALID",
                "checksum chunk count or size mismatch",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoded_object_preserves_canonical_bytes_and_domain_digest() {
        let value = serde_json::json!({"z": [true, null], "a": 1});
        let (bytes, hash) = encode_hashed_object("state-page", &value).unwrap();
        assert_eq!(bytes, br#"{"a":1,"z":[true,null]}"#);
        let mut expected = blake3::Hasher::new();
        expected.update(b"zxs:state-page\0");
        expected.update(br#"{"a":1,"z":[true,null]}"#);
        assert_eq!(hash, expected.finalize().to_hex().as_str());
        assert_eq!(object_hash("state-page", &value).unwrap(), hash);
        assert_ne!(object_hash("snapshot", &value).unwrap(), hash);
        assert_eq!(
            encode_hashed_object("Invalid", &value).unwrap_err().code(),
            "INVALID_OBJECT_TYPE"
        );
        assert!(encode_hashed_object("state-page", &serde_json::json!({"n": u64::MAX})).is_err());
    }
    #[test]
    fn published_vectors() {
        let v: Value =
            serde_json::from_str(include_str!("../../../docs/contracts/vectors.json")).unwrap();
        for c in v["canonical"].as_array().unwrap() {
            let bytes = canonical_json(&c["value"]).unwrap();
            let hex = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
            assert_eq!(hex, c["canonical_utf8_hex"]);
            let prefix = c["prefix_hex"].as_str().unwrap();
            let purpose = match c["name"].as_str().unwrap() {
                "version-jcs" => "version",
                "request-jcs" => "request",
                "utf16-key-order" => "vector",
                name => panic!("unknown canonical vector {name}"),
            };
            let expected_prefix = format!("zxs:{purpose}\0")
                .bytes()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            assert_eq!(prefix, expected_prefix);
            assert_eq!(object_hash(purpose, &c["value"]).unwrap(), c["blake3"]);
            let mut h = blake3::Hasher::new();
            for i in (0..prefix.len()).step_by(2) {
                h.update(&[u8::from_str_radix(&prefix[i..i + 2], 16).unwrap()]);
            }
            h.update(&bytes);
            assert_eq!(
                h.finalize().to_hex().as_str(),
                c["blake3"].as_str().unwrap()
            );
        }
        for c in v["strict_json_negative"].as_array().unwrap() {
            assert!(
                strict_json(c["text"].as_str().unwrap().as_bytes()).is_err(),
                "{}",
                c["name"]
            );
        }
        for c in v["binary"].as_array().unwrap() {
            let s = c["manifest_hex"].as_str().unwrap();
            let b = (0..s.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
                .collect::<Vec<_>>();
            assert_eq!(ChecksumManifest::decode(&b).unwrap().encode().unwrap(), b);
            assert_eq!(
                blake3::hash(&b).to_hex().as_str(),
                c["manifest_hash"].as_str().unwrap()
            );
        }
    }
    #[test]
    fn utf16_order() {
        let v = serde_json::json!({"\u{e000}":1,"\u{10000}":2});
        assert_eq!(
            String::from_utf8(canonical_json(&v).unwrap()).unwrap(),
            "{\"𐀀\":2,\"\":1}"
        );
    }

    #[test]
    fn canonical_json_rejects_float_types_and_unsafe_numeric_integers() {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1.0, 1.5] {
            assert!(canonical_json(&value).is_err());
            assert!(canonical_json(&vec![Some(value)]).is_err());
        }
        assert!(canonical_json(&u64::MAX).is_err());
        assert!(canonical_json(&i64::MIN).is_err());
        assert_eq!(
            canonical_json(&((1u64 << 53) - 1)).unwrap(),
            b"9007199254740991"
        );
        assert_eq!(
            canonical_json(&(-9_007_199_254_740_991i64)).unwrap(),
            b"-9007199254740991"
        );
        assert!(canonical_json(&serde_json::json!({"native_size": u64::MAX})).is_err());
    }

    #[test]
    fn checksum_header_is_semantic_and_rejects_tagged_or_truncated_inputs() {
        let manifest = ChecksumManifest {
            chunk_bytes: 65_536,
            logical_size: 3,
            chunks: vec![*blake3::hash(b"abc").as_bytes()],
        };
        let bytes = manifest.encode().unwrap();
        assert_eq!(bytes.len(), CHECKSUM_HEADER_BYTES + 32);
        assert_eq!(&bytes[..8], b"ZXSCHKS\0");
        assert_eq!(&bytes[8..10], &1u16.to_le_bytes());
        assert_eq!(&bytes[10..14], &65_536u32.to_le_bytes());
        assert_eq!(&bytes[14..22], &3u64.to_le_bytes());
        assert_eq!(&bytes[22..30], &1u64.to_le_bytes());
        for length in 0..bytes.len() {
            assert!(
                ChecksumManifest::decode(&bytes[..length]).is_err(),
                "length {length}"
            );
        }
        let mut unsupported = b"ZXSCHK2\0".to_vec();
        unsupported.extend(2u16.to_le_bytes());
        unsupported.extend_from_slice(&bytes[8..]);
        assert_eq!(
            ChecksumManifest::decode(&unsupported).unwrap_err().code(),
            "CHECKSUM_FORMAT_INVALID"
        );
        let mut wrong_algorithm = bytes.clone();
        wrong_algorithm[8..10].copy_from_slice(&2u16.to_le_bytes());
        assert!(ChecksumManifest::decode(&wrong_algorithm).is_err());
        assert_eq!(
            ChecksumManifest::decode(&bytes).unwrap().encode().unwrap(),
            bytes
        );
    }

    #[test]
    fn checksum_decoder_rejects_count_overflow_extensions_and_unsupported_chunks() {
        let base = ChecksumManifest {
            chunk_bytes: 65536,
            logical_size: 0,
            chunks: vec![],
        };
        assert_eq!(
            ChecksumManifest::decode(&base.encode().unwrap())
                .unwrap()
                .chunk_bytes,
            65536
        );
        let mut corrupt = base.encode().unwrap();
        corrupt[22..30].copy_from_slice(&u64::MAX.to_le_bytes());
        assert!(ChecksumManifest::decode(&corrupt).is_err());
        let mut extension = base.encode().unwrap();
        extension.push(0);
        assert!(ChecksumManifest::decode(&extension).is_err());
        assert!(
            ChecksumManifest {
                chunk_bytes: 16777216,
                ..base
            }
            .encode()
            .is_err()
        );
    }
}
