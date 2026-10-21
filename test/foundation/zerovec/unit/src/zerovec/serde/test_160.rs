#[cfg(test)]
#[allow(non_camel_case_types)]
mod test {
    use crate::samples::*;
    use crate::ZeroVec;

    #[derive(serde::Serialize, serde::Deserialize)]
    #[allow(
        dead_code,
        reason = "Tests compatibility of custom impl with Serde derive."
    )]
    struct DeriveTest_ZeroVec<'data> {
        #[serde(borrow)]
        _data: ZeroVec<'data, u16>,
    }

    #[test]
    fn test_serde_json() {
        let zerovec_orig = ZeroVec::from_slice_or_alloc(TEST_SLICE);
        let json_str = serde_json::to_string(&zerovec_orig).expect("serialize");
        assert_eq!(JSON_STR, json_str);
        // ZeroVec should deserialize from JSON to either Vec or ZeroVec
        let vec_new: Vec<u32> =
            serde_json::from_str(&json_str).expect("deserialize from buffer to Vec");
        assert_eq!(
            zerovec_orig,
            ZeroVec::<u32>::from_slice_or_alloc(vec_new.as_slice())
        );
        let zerovec_new: ZeroVec<u32> =
            serde_json::from_str(&json_str).expect("deserialize from buffer to ZeroVec");
        assert_eq!(zerovec_orig, zerovec_new);
        assert!(zerovec_new.is_owned());
    }

    #[test]
    fn test_serde_bincode() {
        let zerovec_orig = ZeroVec::from_slice_or_alloc(TEST_SLICE);
        let bincode_buf = bincode::serialize(&zerovec_orig).expect("serialize");
        assert_eq!(BINCODE_BUF, bincode_buf);
        // ZeroVec should deserialize from Bincode to ZeroVec but not Vec
        bincode::deserialize::<Vec<u32>>(&bincode_buf).expect_err("deserialize from buffer to Vec");
        let zerovec_new: ZeroVec<u32> =
            bincode::deserialize(&bincode_buf).expect("deserialize from buffer to ZeroVec");
        assert_eq!(zerovec_orig, zerovec_new);

        assert!(!zerovec_new.is_owned());
    }

    #[test]
    fn test_serde_rmp() {
        let zerovec_orig = ZeroVec::from_slice_or_alloc(TEST_SLICE);
        let rmp_buf = rmp_serde::to_vec(&zerovec_orig).expect("serialize");
        // ZeroVec should deserialize from Bincode to ZeroVec but not Vec
        bincode::deserialize::<Vec<u32>>(&rmp_buf).expect_err("deserialize from buffer to Vec");
        let zerovec_new: ZeroVec<u32> =
            rmp_serde::from_slice(&rmp_buf).expect("deserialize from buffer to ZeroVec");
        assert_eq!(zerovec_orig, zerovec_new);

        assert!(!zerovec_new.is_owned());
    }

    #[test]
    fn test_chars_valid() {
        // 1-byte, 2-byte, 3-byte, and 4-byte character in UTF-8 (not as relevant in UTF-32)
        let zerovec_orig = ZeroVec::alloc_from_slice(&['w', 'ω', '文', '𑄃']);
        let bincode_buf = bincode::serialize(&zerovec_orig).expect("serialize");
        let zerovec_new: ZeroVec<char> =
            bincode::deserialize(&bincode_buf).expect("deserialize from buffer to ZeroVec");
        assert_eq!(zerovec_orig, zerovec_new);

        assert!(!zerovec_new.is_owned());
    }

    #[test]
    fn test_chars_invalid() {
        // 119 and 120 are valid, but not 0xD800 (high surrogate)
        let zerovec_orig: ZeroVec<u32> = ZeroVec::from_slice_or_alloc(&[119, 0xD800, 120]);
        let bincode_buf = bincode::serialize(&zerovec_orig).expect("serialize");
        let zerovec_result = bincode::deserialize::<ZeroVec<char>>(&bincode_buf);
        assert!(zerovec_result.is_err());
    }
}
