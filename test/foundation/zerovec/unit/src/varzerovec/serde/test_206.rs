#[cfg(test)]
#[allow(non_camel_case_types)]
mod test {
    use crate::{VarZeroSlice, VarZeroVec};

    #[derive(serde::Serialize, serde::Deserialize)]
    #[allow(
        dead_code,
        reason = "Tests compatibility of custom impl with Serde derive."
    )]
    struct DeriveTest_VarZeroVec<'data> {
        #[serde(borrow)]
        _data: VarZeroVec<'data, str>,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    #[allow(
        dead_code,
        reason = "Tests compatibility of custom impl with Serde derive."
    )]
    struct DeriveTest_VarZeroSlice<'data> {
        #[serde(borrow)]
        _data: &'data VarZeroSlice<str>,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    #[allow(
        dead_code,
        reason = "Tests compatibility of custom impl with Serde derive."
    )]
    struct DeriveTest_VarZeroVec_of_VarZeroSlice<'data> {
        #[serde(borrow)]
        _data: VarZeroVec<'data, VarZeroSlice<str>>,
    }

    // ["foo", "bar", "baz", "dolor", "quux", "lorem ipsum"];
    const BYTES: &[u8] = &[
        6, 0, 3, 0, 6, 0, 9, 0, 14, 0, 18, 0, 102, 111, 111, 98, 97, 114, 98, 97, 122, 100, 111,
        108, 111, 114, 113, 117, 117, 120, 108, 111, 114, 101, 109, 32, 105, 112, 115, 117, 109,
    ];
    const JSON_STR: &str = "[\"foo\",\"bar\",\"baz\",\"dolor\",\"quux\",\"lorem ipsum\"]";
    const BINCODE_BUF: &[u8] = &[
        41, 0, 0, 0, 0, 0, 0, 0, 6, 0, 3, 0, 6, 0, 9, 0, 14, 0, 18, 0, 102, 111, 111, 98, 97, 114,
        98, 97, 122, 100, 111, 108, 111, 114, 113, 117, 117, 120, 108, 111, 114, 101, 109, 32, 105,
        112, 115, 117, 109,
    ];

    // ["w", "ω", "文", "𑄃"]
    const NONASCII_STR: &[&str] = &["w", "ω", "文", "𑄃"];
    const NONASCII_BYTES: &[u8] = &[
        4, 0, 1, 0, 3, 0, 6, 0, 119, 207, 137, 230, 150, 135, 240, 145, 132, 131,
    ];
    #[test]
    fn test_serde_json() {
        let zerovec_orig: VarZeroVec<str> = VarZeroVec::parse_bytes(BYTES).expect("parse");
        let json_str = serde_json::to_string(&zerovec_orig).expect("serialize");
        assert_eq!(JSON_STR, json_str);
        // VarZeroVec should deserialize from JSON to either Vec or VarZeroVec
        let vec_new: Vec<Box<str>> =
            serde_json::from_str(&json_str).expect("deserialize from buffer to Vec");
        assert_eq!(zerovec_orig.to_vec(), vec_new);
        let zerovec_new: VarZeroVec<str> =
            serde_json::from_str(&json_str).expect("deserialize from buffer to VarZeroVec");
        assert_eq!(zerovec_orig.to_vec(), zerovec_new.to_vec());
        assert!(zerovec_new.is_owned());
    }

    #[test]
    fn test_serde_bincode() {
        let zerovec_orig: VarZeroVec<str> = VarZeroVec::parse_bytes(BYTES).expect("parse");
        let bincode_buf = bincode::serialize(&zerovec_orig).expect("serialize");
        assert_eq!(BINCODE_BUF, bincode_buf);
        let zerovec_new: VarZeroVec<str> =
            bincode::deserialize(&bincode_buf).expect("deserialize from buffer to VarZeroVec");
        assert_eq!(zerovec_orig.to_vec(), zerovec_new.to_vec());
        assert!(!zerovec_new.is_owned());
    }

    #[test]
    fn test_vzv_borrowed() {
        let zerovec_orig: &VarZeroSlice<str> = VarZeroSlice::parse_bytes(BYTES).expect("parse");
        let bincode_buf = bincode::serialize(&zerovec_orig).expect("serialize");
        assert_eq!(BINCODE_BUF, bincode_buf);
        let zerovec_new: &VarZeroSlice<str> =
            bincode::deserialize(&bincode_buf).expect("deserialize from buffer to VarZeroSlice");
        assert_eq!(zerovec_orig.to_vec(), zerovec_new.to_vec());
    }

    #[test]
    fn test_nonascii_bincode() {
        let src_vec = NONASCII_STR
            .iter()
            .copied()
            .map(Box::<str>::from)
            .collect::<Vec<_>>();
        let mut zerovec: VarZeroVec<str> = VarZeroVec::parse_bytes(NONASCII_BYTES).expect("parse");
        assert_eq!(zerovec.to_vec(), src_vec);
        let bincode_buf = bincode::serialize(&zerovec).expect("serialize");
        let zerovec_result =
            bincode::deserialize::<VarZeroVec<str>>(&bincode_buf).expect("deserialize");
        assert_eq!(zerovec_result.to_vec(), src_vec);

        // try again with owned zerovec
        zerovec.make_mut();
        let bincode_buf = bincode::serialize(&zerovec).expect("serialize");
        let zerovec_result =
            bincode::deserialize::<VarZeroVec<str>>(&bincode_buf).expect("deserialize");
        assert_eq!(zerovec_result.to_vec(), src_vec);
    }
}
