#[cfg(test)]
mod tests {
    use super::*;
    use tinystr::tinystr;

    #[test]
    fn test_subtag() {
        let subtag = subtag!("foo");
        assert_eq!(subtag.as_str(), "foo");
    }

    #[test]
    fn test_subtag_from_tinystr() {
        let subtag = Subtag::try_from(tinystr!(3, "foo"));
        assert!(subtag.is_ok());

        let subtag = Subtag::try_from(tinystr!(1, "f"));
        assert!(subtag.is_err());
    }
}
