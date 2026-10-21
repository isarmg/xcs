#[cfg(all(test, feature = "serialization"))]
mod test {
    use super::*;

    #[test]
    fn test_serialization() {
        let mut set = AHashSet::new();
        set.insert("for".to_string());
        set.insert("bar".to_string());
        let mut serialization = json_codec::to_string(&set).unwrap();
        let mut deserialization: AHashSet<String> = json_codec::from_str(&serialization).unwrap();
        assert_eq!(deserialization, set);

        set.insert("baz".to_string());
        serialization = json_codec::to_string(&set).unwrap();
        let mut deserializer = json_codec::Deserializer::from_str(&serialization);
        AHashSet::deserialize_in_place(&mut deserializer, &mut deserialization).unwrap();
        assert_eq!(deserialization, set);
    }
}
