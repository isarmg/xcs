#[cfg(test)]
#[cfg(feature = "zerovec")]
mod tests_zerovec {
    use super::*;
    use data_serialization::{Deserialize, Serialize};
    use zerovec::ZeroVec;

    #[derive(Serialize, Deserialize)]
    pub struct ZeroTrieSimpleAsciiZeroVec<'a> {
        #[serialization(borrow)]
        trie: ZeroTrieSimpleAscii<ZeroVec<'a, u8>>,
    }

    #[test]
    pub fn test_serialization_simpleascii_zerovec() {
        let trie =
            ZeroTrieSimpleAscii::from_store(ZeroVec::new_borrowed(testdata::basic::TRIE_ASCII));
        let original = ZeroTrieSimpleAsciiZeroVec { trie };
        let json_str = json_codec::to_string(&original).unwrap();
        let bincode_bytes = bincode::serialize(&original).unwrap();

        assert_eq!(json_str, testdata::basic::JSON_STR_ASCII);
        assert_eq!(&bincode_bytes[0..9], &[0, 26, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(&bincode_bytes[9..], testdata::basic::BINCODE_BYTES_ASCII);

        let json_recovered: ZeroTrieSimpleAsciiZeroVec = json_codec::from_str(&json_str).unwrap();
        let bincode_recovered: ZeroTrieSimpleAsciiZeroVec =
            bincode::deserialize(&bincode_bytes).unwrap();

        assert_eq!(original.trie, json_recovered.trie);
        assert_eq!(original.trie, bincode_recovered.trie);

        assert!(json_recovered.trie.into_store().is_owned());
        assert!(!bincode_recovered.trie.into_store().is_owned());
    }

    #[derive(Serialize, Deserialize)]
    pub struct ZeroTriePerfectHashZeroVec<'a> {
        #[serialization(borrow)]
        trie: ZeroTriePerfectHash<ZeroVec<'a, u8>>,
    }

    #[test]
    pub fn test_serialization_perfecthash_zerovec() {
        let trie =
            ZeroTriePerfectHash::from_store(ZeroVec::new_borrowed(testdata::basic::TRIE_ASCII));
        let original = ZeroTriePerfectHashZeroVec { trie };
        let json_str = json_codec::to_string(&original).unwrap();
        let bincode_bytes = bincode::serialize(&original).unwrap();

        assert_eq!(json_str, testdata::basic::JSON_STR_ASCII);
        assert_eq!(&bincode_bytes[0..9], &[3, 26, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(&bincode_bytes[9..], testdata::basic::BINCODE_BYTES_ASCII);

        let json_recovered: ZeroTriePerfectHashZeroVec = json_codec::from_str(&json_str).unwrap();
        let bincode_recovered: ZeroTriePerfectHashZeroVec =
            bincode::deserialize(&bincode_bytes).unwrap();

        assert_eq!(original.trie, json_recovered.trie);
        assert_eq!(original.trie, bincode_recovered.trie);

        assert!(json_recovered.trie.into_store().is_owned());
        assert!(!bincode_recovered.trie.into_store().is_owned());
    }
}
