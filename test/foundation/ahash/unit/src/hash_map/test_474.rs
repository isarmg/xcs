#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_borrow() {
        let mut map: AHashMap<String, String> = AHashMap::new();
        map.insert("foo".to_string(), "Bar".to_string());
        map.insert("Bar".to_string(), map.get("foo").unwrap().to_owned());
    }

    #[cfg(feature = "serialization")]
    #[test]
    fn test_serialization() {
        let mut map = AHashMap::new();
        map.insert("for".to_string(), 0);
        map.insert("bar".to_string(), 1);
        let mut serialization = json_codec::to_string(&map).unwrap();
        let mut deserialization: AHashMap<String, u64> = json_codec::from_str(&serialization).unwrap();
        assert_eq!(deserialization, map);

        map.insert("baz".to_string(), 2);
        serialization = json_codec::to_string(&map).unwrap();
        let mut deserializer = json_codec::Deserializer::from_str(&serialization);
        AHashMap::deserialize_in_place(&mut deserializer, &mut deserialization).unwrap();
        assert_eq!(deserialization, map);
    }
}
