#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debugs_set() {
        let mut m: Set<String, 10> = Set::new();
        m.insert("one".to_string());
        m.insert("two".to_string());
        assert_eq!(r#"{"one", "two"}"#, format!("{:?}", m));
    }

    #[test]
    fn debug_alternate_set() {
        let mut m: Set<String, 10> = Set::new();
        m.insert("one".to_string());
        m.insert("two".to_string());
        assert_eq!(
            r#"{
    "one",
    "two",
}"#,
            format!("{:#?}", m)
        );
    }
}
