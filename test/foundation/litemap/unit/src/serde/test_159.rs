#[cfg(test)]
mod test {
    use crate::LiteMap;
    use alloc::borrow::ToOwned;
    use alloc::string::String;

    fn get_simple_map() -> LiteMap<u32, String> {
        [
            (1, "one".to_owned()),
            (2, "two".to_owned()),
            (4, "four".to_owned()),
            (5, "five".to_owned()),
        ]
        .into_iter()
        .collect()
    }

    fn get_tuple_map() -> LiteMap<(u32, String), String> {
        [
            ((1, "en".to_owned()), "one".to_owned()),
            ((1, "zh".to_owned()), "一".to_owned()),
            ((2, "en".to_owned()), "two".to_owned()),
            ((2, "zh".to_owned()), "二".to_owned()),
            ((4, "en".to_owned()), "four".to_owned()),
            ((5, "en".to_owned()), "five".to_owned()),
            ((5, "zh".to_owned()), "五".to_owned()),
            ((7, "zh".to_owned()), "七".to_owned()),
        ]
        .into_iter()
        .collect()
    }

    #[test]
    fn test_roundtrip_json() {
        let map = get_simple_map();
        let json = serde_json::to_string(&map).unwrap();
        assert_eq!(
            json,
            "{\"1\":\"one\",\"2\":\"two\",\"4\":\"four\",\"5\":\"five\"}"
        );
        let deserialized: LiteMap<u32, String> = serde_json::from_str(&json).unwrap();
        assert_eq!(map, deserialized);

        let map = get_tuple_map();
        let json = serde_json::to_string(&map).unwrap();
        assert_eq!(
            json,
            "[[[1,\"en\"],\"one\"],[[1,\"zh\"],\"一\"],[[2,\"en\"],\"two\"],\
                          [[2,\"zh\"],\"二\"],[[4,\"en\"],\"four\"],[[5,\"en\"],\"five\"],\
                          [[5,\"zh\"],\"五\"],[[7,\"zh\"],\"七\"]]"
        );
        let deserialized: LiteMap<(u32, String), String> = serde_json::from_str(&json).unwrap();
        assert_eq!(map, deserialized);
    }

    #[test]
    fn test_roundtrip_postcard() {
        let map = get_simple_map();
        let postcard = postcard::to_stdvec(&map).unwrap();
        let deserialized: LiteMap<u32, String> = postcard::from_bytes(&postcard).unwrap();
        assert_eq!(map, deserialized);

        let map = get_tuple_map();
        let postcard = postcard::to_stdvec(&map).unwrap();
        let deserialized: LiteMap<(u32, String), String> = postcard::from_bytes(&postcard).unwrap();
        assert_eq!(map, deserialized);
    }

    /// Test that a [`LiteMap<_, _, Vec>`] is deserialized with an exact capacity
    /// if the deserializer provides a size hint information, like postcard here.
    #[test]
    fn test_deserialize_capacity() {
        for len in 0..50 {
            let mut map = (0..len).map(|i| (i, i.to_string())).collect::<Vec<_>>();
            let postcard = postcard::to_stdvec(&map).unwrap();
            let deserialized: LiteMap<u32, String> = postcard::from_bytes(&postcard).unwrap();
            assert_eq!(deserialized.values.capacity(), len);
            assert_eq!(deserialized.values.len(), len);
            // again, but with a shuffled map
            rand::seq::SliceRandom::shuffle(&mut map[..], &mut rand::rng());
            let postcard = postcard::to_stdvec(&map).unwrap();
            let deserialized: LiteMap<u32, String> = postcard::from_bytes(&postcard).unwrap();
            assert_eq!(deserialized.values.capacity(), len);
            assert_eq!(deserialized.values.len(), len);
        }
    }
}
