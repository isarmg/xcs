use super::*;

#[test]
fn encoded_object_preserves_canonical_bytes_and_domain_digest() {
    let value = json_codec::json!({"z": [true, null], "a": 1});
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
    assert!(encode_hashed_object("state-page", &json_codec::json!({"n": u64::MAX})).is_err());
}
#[test]
fn published_vectors() {
    let v: Value =
        json_codec::from_str(include_str!("../../../../docs/contracts/vectors.json")).unwrap();
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
    let v = json_codec::json!({"\u{e000}":1,"\u{10000}":2});
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
    assert!(canonical_json(&json_codec::json!({"native_size": u64::MAX})).is_err());
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
