#[cfg(test)]
#[allow(non_camel_case_types)]
mod test {
    use crate::{map::ZeroMapBorrowed, ZeroMap};

    #[derive(data_serialization::Serialize, data_serialization::Deserialize)]
    #[expect(
        dead_code,
        reason = "Tests compatibility of custom impl with Serde derive."
    )]
    struct DeriveTest_ZeroMap<'data> {
        #[serialization(borrow)]
        _data: ZeroMap<'data, str, [u8]>,
    }

    #[derive(data_serialization::Serialize, data_serialization::Deserialize)]
    #[expect(
        dead_code,
        reason = "Tests compatibility of custom impl with Serde derive."
    )]
    struct DeriveTest_ZeroMapBorrowed<'data> {
        #[serialization(borrow)]
        _data: ZeroMapBorrowed<'data, str, [u8]>,
    }

    const JSON_STR: &str = "{\"1\":\"uno\",\"2\":\"dos\",\"3\":\"tres\"}";
    const BINCODE_BYTES: &[u8] = &[
        12, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0, 3, 0,
        3, 0, 6, 0, 117, 110, 111, 100, 111, 115, 116, 114, 101, 115,
    ];

    fn make_map() -> ZeroMap<'static, u32, str> {
        let mut map = ZeroMap::new();
        map.insert(&1, "uno");
        map.insert(&2, "dos");
        map.insert(&3, "tres");
        map
    }

    #[test]
    fn test_serialization_json() {
        let map = make_map();
        let json_str = json_codec::to_string(&map).expect("serialize");
        assert_eq!(JSON_STR, json_str);
        let new_map: ZeroMap<u32, str> = json_codec::from_str(&json_str).expect("deserialize");
        assert_eq!(
            new_map.iter().collect::<Vec<_>>(),
            map.iter().collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_serialization_json_complex_key() {
        let mut map = ZeroMap::new();
        map.insert(&(1, 1), "uno");
        map.insert(&(2, 2), "dos");
        map.insert(&(3, 3), "tres");
        let json_str = json_codec::to_string(&map).expect("serialize");
        assert_eq!(
            json_str,
            "[[[1,1],\"uno\"],[[2,2],\"dos\"],[[3,3],\"tres\"]]"
        );
        let new_map: ZeroMap<(u32, u32), str> =
            json_codec::from_str(&json_str).expect("deserialize");
        assert_eq!(
            new_map.iter().collect::<Vec<_>>(),
            map.iter().collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_bincode() {
        let map = make_map();
        let bincode_bytes = bincode::serialize(&map).expect("serialize");
        assert_eq!(BINCODE_BYTES, bincode_bytes);
        let new_map: ZeroMap<u32, str> = bincode::deserialize(&bincode_bytes).expect("deserialize");
        assert_eq!(
            new_map.iter().collect::<Vec<_>>(),
            map.iter().collect::<Vec<_>>()
        );

        let new_map: ZeroMapBorrowed<u32, str> =
            bincode::deserialize(&bincode_bytes).expect("deserialize");
        assert_eq!(
            new_map.iter().collect::<Vec<_>>(),
            map.iter().collect::<Vec<_>>()
        );
    }
}
