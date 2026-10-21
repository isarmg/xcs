#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iterate_keys() {
        let mut m: Map<String, i32, 10> = Map::new();
        m.insert("foo".to_string(), 0);
        m.insert("bar".to_string(), 0);
        let keys = m.keys();
        assert_eq!(keys.len(), 2);
        assert_eq!(keys.collect::<Vec<_>>(), [&"foo", &"bar"]);
    }

    #[test]
    fn iterate_into_keys() {
        let mut m: Map<String, i32, 10> = Map::new();
        m.insert("foo".to_string(), 0);
        m.insert("bar".to_string(), 0);
        let keys = m.into_keys();
        assert_eq!(keys.len(), 2);
        assert_eq!(
            keys.collect::<Vec<_>>(),
            ["bar".to_string(), "foo".to_string()]
        );
    }

    #[test]
    fn keys_clone() {
        let mut m: Map<String, i32, 10> = Map::new();
        m.insert("foo".to_string(), 0);
        m.insert("bar".to_string(), 0);
        let keys = m.keys();
        let cloned_keys = keys.clone();
        assert_eq!(keys.collect::<Vec<_>>(), cloned_keys.collect::<Vec<_>>());
    }

    #[test]
    fn keys_debug() {
        let mut m: Map<String, i32, 10> = Map::new();
        m.insert("foo".to_string(), 0);
        m.insert("bar".to_string(), 0);
        let keys = m.keys();
        let debug_output = format!("{:?}", keys);
        assert!(debug_output.contains("foo"));
        assert!(debug_output.contains("bar"));
    }

    #[test]
    fn keys_default() {
        let keys: Keys<'_, String, i32> = Keys::default();
        assert_eq!(keys.len(), 0);
    }

    #[test]
    fn into_keys_debug() {
        let mut m: Map<String, i32, 10> = Map::new();
        m.insert("foo".to_string(), 0);
        m.insert("bar".to_string(), 0);
        let into_keys = m.into_keys();
        let debug_output = format!("{:?}", into_keys);
        assert!(debug_output.contains("foo"));
        assert!(debug_output.contains("bar"));
    }

    #[test]
    fn into_keys_default() {
        let into_keys: IntoKeys<String, i32, 10> = IntoKeys::default();
        assert_eq!(into_keys.len(), 0);
    }
}
