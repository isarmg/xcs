#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debugs_map() {
        let mut m: Map<String, i32, 10> = Map::new();
        m.insert("one".to_string(), 42);
        m.insert("two".to_string(), 16);
        assert_eq!(r#"{"one": 42, "two": 16}"#, format!("{:?}", m));
    }

    #[test]
    fn debug_alternate_map() {
        let mut m: Map<String, i32, 10> = Map::new();
        m.insert("one".to_string(), 42);
        m.insert("two".to_string(), 16);
        assert_eq!(
            r#"{
    "one": 42,
    "two": 16,
}"#,
            format!("{:#?}", m)
        );
    }
}
