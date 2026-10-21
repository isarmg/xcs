#[cfg(test)]
#[allow(non_camel_case_types)]
mod test {
    use crate::map2d::{ZeroMap2d, ZeroMap2dBorrowed};

    #[derive(data_serialization::Serialize, data_serialization::Deserialize)]
    #[allow(
        dead_code,
        reason = "We are testing that these types can be deserialized, and Tests compatibility of custom impl with Serde derive."
    )]
    struct DeriveTest_ZeroMap2d<'data> {
        #[serialization(borrow)]
        _data: ZeroMap2d<'data, u16, str, [u8]>,
    }

    #[derive(data_serialization::Serialize, data_serialization::Deserialize)]
    #[allow(
        dead_code,
        reason = "We are testing that these types can be deserialized, and Tests compatibility of custom impl with Serde derive."
    )]
    struct DeriveTest_ZeroMap2dBorrowed<'data> {
        #[serialization(borrow)]
        _data: ZeroMap2dBorrowed<'data, u16, str, [u8]>,
    }

    const JSON_STR: &str = "{\"1\":{\"1\":\"uno\"},\"2\":{\"2\":\"dos\",\"3\":\"tres\"}}";
    const BINCODE_BYTES: &[u8] = &[
        8, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 3, 0,
        0, 0, 6, 0, 0, 0, 0, 0, 0, 0, 1, 0, 2, 0, 3, 0, 16, 0, 0, 0, 0, 0, 0, 0, 3, 0, 3, 0, 6, 0,
        117, 110, 111, 100, 111, 115, 116, 114, 101, 115,
    ];

    fn make_map() -> ZeroMap2d<'static, u32, u16, str> {
        let mut map = ZeroMap2d::new();
        map.insert(&1, &1, "uno");
        map.insert(&2, &2, "dos");
        map.insert(&2, &3, "tres");
        map
    }

    #[test]
    fn test_serialization_json() {
        let map = make_map();
        let json_str = json_codec::to_string(&map).expect("serialize");
        assert_eq!(JSON_STR, json_str);
        let new_map: ZeroMap2d<u32, u16, str> =
            json_codec::from_str(&json_str).expect("deserialize");
        assert_eq!(format!("{new_map:?}"), format!("{map:?}"));
    }

    #[test]
    fn test_bincode() {
        let map = make_map();
        let bincode_bytes = bincode::serialize(&map).expect("serialize");
        assert_eq!(BINCODE_BYTES, bincode_bytes);
        let new_map: ZeroMap2d<u32, u16, str> =
            bincode::deserialize(&bincode_bytes).expect("deserialize");
        assert_eq!(
            format!("{new_map:?}"),
            format!("{map:?}").replace("Owned", "Borrowed"),
        );

        let new_map: ZeroMap2dBorrowed<u32, u16, str> =
            bincode::deserialize(&bincode_bytes).expect("deserialize");
        assert_eq!(
            format!("{new_map:?}"),
            format!("{map:?}")
                .replace("Owned", "Borrowed")
                .replace("ZeroMap2d", "ZeroMap2dBorrowed")
        );
    }

    #[test]
    fn test_serialization_rmp() {
        let map = make_map();
        let rmp_buf = rmp_serialization::to_vec(&map).expect("serialize");
        let new_map: ZeroMap2d<u32, u16, str> = rmp_serialization::from_slice(&rmp_buf).unwrap();
        assert_eq!(map, new_map);
    }

    #[test]
    fn test_sample_bincode() {
        // This is the map from the main docs page for ZeroMap2d
        let mut map: ZeroMap2d<u16, u16, str> = ZeroMap2d::new();
        map.insert(&1, &2, "three");
        let bincode_bytes: Vec<u8> = bincode::serialize(&map).expect("serialize");
        assert_eq!(
            bincode_bytes.as_slice(),
            &[
                2, 0, 0, 0, 0, 0, 0, 0, 1, 0, 4, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 0, 0,
                0, 0, 2, 0, 7, 0, 0, 0, 0, 0, 0, 0, 1, 0, 116, 104, 114, 101, 101
            ]
        );
    }
}
