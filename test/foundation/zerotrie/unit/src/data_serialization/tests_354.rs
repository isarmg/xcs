#[cfg(test)]
mod tests {
    use super::*;
    use alloc::borrow::Cow;
    use data_serialization::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub struct ZeroTrieSimpleAsciiCow<'a> {
        #[serialization(borrow)]
        trie: ZeroTrieSimpleAscii<Cow<'a, [u8]>>,
    }

    #[test]
    pub fn test_serialization_simpleascii_cow() {
        let trie = ZeroTrieSimpleAscii::from_store(Cow::from(testdata::basic::TRIE_ASCII));
        let original = ZeroTrieSimpleAsciiCow { trie };
        let json_str = json_codec::to_string(&original).unwrap();
        let bincode_bytes = bincode::serialize(&original).unwrap();
        let rmp_bytes = rmp_serialization::to_vec(&original).unwrap();

        assert_eq!(json_str, testdata::basic::JSON_STR_ASCII);
        assert_eq!(&bincode_bytes[0..9], &[0, 26, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(&bincode_bytes[9..], testdata::basic::BINCODE_BYTES_ASCII);
        assert_eq!(&rmp_bytes[0..5], &[145, 146, 0, 196, 26]);
        assert_eq!(&rmp_bytes[5..], testdata::basic::BINCODE_BYTES_ASCII);

        let json_recovered: ZeroTrieSimpleAsciiCow = json_codec::from_str(&json_str).unwrap();
        let bincode_recovered: ZeroTrieSimpleAsciiCow =
            bincode::deserialize(&bincode_bytes).unwrap();
        let rmp_recovered: ZeroTrieSimpleAsciiCow = rmp_serialization::from_slice(&rmp_bytes).unwrap();

        assert_eq!(original.trie, json_recovered.trie);
        assert_eq!(original.trie, bincode_recovered.trie);
        assert_eq!(original.trie, rmp_recovered.trie);

        assert!(matches!(json_recovered.trie.into_store(), Cow::Owned(_)));
        assert!(matches!(
            bincode_recovered.trie.into_store(),
            Cow::Borrowed(_)
        ));
    }

    #[derive(Serialize, Deserialize)]
    pub struct ZeroAsciiIgnoreCaseTrieCow<'a> {
        #[serialization(borrow)]
        trie: ZeroAsciiIgnoreCaseTrie<Cow<'a, [u8]>>,
    }

    #[test]
    pub fn test_serialization_asciiignorecase_cow() {
        let trie = ZeroAsciiIgnoreCaseTrie::from_store(Cow::from(testdata::basic::TRIE_ASCII));
        let original = ZeroAsciiIgnoreCaseTrieCow { trie };
        let json_str = json_codec::to_string(&original).unwrap();
        let bincode_bytes = bincode::serialize(&original).unwrap();

        assert_eq!(json_str, testdata::basic::JSON_STR_ASCII);
        assert_eq!(&bincode_bytes[0..9], &[8, 26, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(&bincode_bytes[9..], testdata::basic::BINCODE_BYTES_ASCII);

        let json_recovered: ZeroAsciiIgnoreCaseTrieCow = json_codec::from_str(&json_str).unwrap();
        let bincode_recovered: ZeroAsciiIgnoreCaseTrieCow =
            bincode::deserialize(&bincode_bytes).unwrap();

        assert_eq!(original.trie, json_recovered.trie);
        assert_eq!(original.trie, bincode_recovered.trie);

        assert!(matches!(json_recovered.trie.into_store(), Cow::Owned(_)));
        assert!(matches!(
            bincode_recovered.trie.into_store(),
            Cow::Borrowed(_)
        ));
    }

    #[derive(Serialize, Deserialize)]
    pub struct ZeroTriePerfectHashCow<'a> {
        #[serialization(borrow)]
        trie: ZeroTriePerfectHash<Cow<'a, [u8]>>,
    }

    #[test]
    pub fn test_serialization_perfecthash_cow() {
        let trie = ZeroTriePerfectHash::from_store(Cow::from(testdata::basic::TRIE_ASCII));
        let original = ZeroTriePerfectHashCow { trie };
        let json_str = json_codec::to_string(&original).unwrap();
        let bincode_bytes = bincode::serialize(&original).unwrap();

        assert_eq!(json_str, testdata::basic::JSON_STR_ASCII);
        assert_eq!(&bincode_bytes[0..9], &[3, 26, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(&bincode_bytes[9..], testdata::basic::BINCODE_BYTES_ASCII);

        let json_recovered: ZeroTriePerfectHashCow = json_codec::from_str(&json_str).unwrap();
        let bincode_recovered: ZeroTriePerfectHashCow =
            bincode::deserialize(&bincode_bytes).unwrap();

        assert_eq!(original.trie, json_recovered.trie);
        assert_eq!(original.trie, bincode_recovered.trie);

        assert!(matches!(json_recovered.trie.into_store(), Cow::Owned(_)));
        assert!(matches!(
            bincode_recovered.trie.into_store(),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    pub fn test_serialization_perfecthash_cow_u() {
        let trie = ZeroTriePerfectHash::from_store(Cow::from(testdata::basic::TRIE_UNICODE));
        let original = ZeroTriePerfectHashCow { trie };
        let json_str = json_codec::to_string(&original).unwrap();
        let bincode_bytes = bincode::serialize(&original).unwrap();

        assert_eq!(json_str, testdata::basic::JSON_STR_UNICODE);
        assert_eq!(&bincode_bytes[0..9], &[3, 39, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(&bincode_bytes[9..], testdata::basic::BINCODE_BYTES_UNICODE);

        let json_recovered: ZeroTriePerfectHashCow = json_codec::from_str(&json_str).unwrap();
        let bincode_recovered: ZeroTriePerfectHashCow =
            bincode::deserialize(&bincode_bytes).unwrap();

        assert_eq!(original.trie, json_recovered.trie);
        assert_eq!(original.trie, bincode_recovered.trie);

        assert!(matches!(json_recovered.trie.into_store(), Cow::Owned(_)));
        assert!(matches!(
            bincode_recovered.trie.into_store(),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    pub fn test_serialization_perfecthash_cow_bin() {
        let trie = ZeroTriePerfectHash::from_store(Cow::from(testdata::basic::TRIE_BINARY));
        let original = ZeroTriePerfectHashCow { trie };
        let json_str = json_codec::to_string(&original).unwrap();
        let bincode_bytes = bincode::serialize(&original).unwrap();

        assert_eq!(json_str, testdata::basic::JSON_STR_BINARY);
        assert_eq!(&bincode_bytes[0..9], &[3, 26, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(&bincode_bytes[9..], testdata::basic::BINCODE_BYTES_BINARY);

        let json_recovered: ZeroTriePerfectHashCow = json_codec::from_str(&json_str).unwrap();
        let bincode_recovered: ZeroTriePerfectHashCow =
            bincode::deserialize(&bincode_bytes).unwrap();

        assert_eq!(original.trie, json_recovered.trie);
        assert_eq!(original.trie, bincode_recovered.trie);

        assert!(matches!(json_recovered.trie.into_store(), Cow::Owned(_)));
        assert!(matches!(
            bincode_recovered.trie.into_store(),
            Cow::Borrowed(_)
        ));
    }

    #[derive(Serialize, Deserialize)]
    pub struct ZeroTrieAnyCow<'a> {
        #[serialization(borrow)]
        trie: ZeroTrie<Cow<'a, [u8]>>,
    }

    #[test]
    pub fn test_serialization_any_cow() {
        let trie =
            ZeroTrieSimpleAscii::from_store(Cow::from(testdata::basic::TRIE_ASCII)).into_zerotrie();
        let original = ZeroTrieAnyCow { trie };
        let json_str = json_codec::to_string(&original).unwrap();
        let bincode_bytes = bincode::serialize(&original).unwrap();

        assert_eq!(json_str, testdata::basic::JSON_STR_ASCII);
        assert_eq!(&bincode_bytes[0..9], &[27, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(&bincode_bytes[9..], testdata::basic::BINCODE_BYTES_ASCII);

        let json_recovered: ZeroTrieAnyCow = json_codec::from_str(&json_str).unwrap();
        let bincode_recovered: ZeroTrieAnyCow = bincode::deserialize(&bincode_bytes).unwrap();

        assert_eq!(original.trie, json_recovered.trie);
        assert_eq!(original.trie, bincode_recovered.trie);

        assert!(matches!(json_recovered.trie.into_store(), Cow::Owned(_)));
        assert!(matches!(
            bincode_recovered.trie.into_store(),
            Cow::Borrowed(_)
        ));
    }

    #[test]
    pub fn test_serialization_any_cow_u() {
        let trie = ZeroTriePerfectHash::from_store(Cow::from(testdata::basic::TRIE_UNICODE))
            .into_zerotrie();
        let original = ZeroTrieAnyCow { trie };
        let json_str = json_codec::to_string(&original).unwrap();
        let bincode_bytes = bincode::serialize(&original).unwrap();

        assert_eq!(json_str, testdata::basic::JSON_STR_UNICODE);
        assert_eq!(&bincode_bytes[0..9], &[40, 0, 0, 0, 0, 0, 0, 0, 3]);
        assert_eq!(&bincode_bytes[9..], testdata::basic::BINCODE_BYTES_UNICODE);

        let json_recovered: ZeroTrieAnyCow = json_codec::from_str(&json_str).unwrap();
        let bincode_recovered: ZeroTrieAnyCow = bincode::deserialize(&bincode_bytes).unwrap();

        assert_eq!(original.trie, json_recovered.trie);
        assert_eq!(original.trie, bincode_recovered.trie);

        assert!(matches!(json_recovered.trie.into_store(), Cow::Owned(_)));
        assert!(matches!(
            bincode_recovered.trie.into_store(),
            Cow::Borrowed(_)
        ));
    }
}
