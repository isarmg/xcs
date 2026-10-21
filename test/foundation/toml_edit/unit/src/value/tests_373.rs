#[cfg(test)]
#[cfg(feature = "parse")]
#[cfg(feature = "display")]
mod tests {
    use super::*;

    #[test]
    fn from_iter_formatting() {
        let features = ["node".to_owned(), "mouth".to_owned()];
        let features: Value = features.iter().cloned().collect();
        assert_eq!(features.to_string(), r#"["node", "mouth"]"#);
    }
}
