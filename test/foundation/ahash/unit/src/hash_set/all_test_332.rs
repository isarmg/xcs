#[cfg(all(test, feature = "serde"))]
mod test {
    use super::*;

    #[test]
    fn test_serde() {
        let mut set = AHashSet::new();
        set.insert("for".to_string());
        set.insert("bar".to_string());
        let mut serialization = serde_json::to_string(&set).unwrap();
        let mut deserialization: AHashSet<String> = serde_json::from_str(&serialization).unwrap();
        assert_eq!(deserialization, set);

        set.insert("baz".to_string());
        serialization = serde_json::to_string(&set).unwrap();
        let mut deserializer = serde_json::Deserializer::from_str(&serialization);
        AHashSet::deserialize_in_place(&mut deserializer, &mut deserialization).unwrap();
        assert_eq!(deserialization, set);
    }
}
