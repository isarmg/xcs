#[cfg(test)]
mod tests {
    use super::{Error, FromIterator, Ident, ParamsString, Value};

    #[cfg(feature = "alloc")]
    use alloc::string::ToString;
    use core::str::FromStr;

    #[test]
    fn add() {
        let mut params = ParamsString::new();
        params.add_str("a", "1").unwrap();
        params.add_decimal("b", 2).unwrap();
        params.add_str("c", "3").unwrap();

        assert_eq!(params.iter().count(), 3);
        assert_eq!(params.get_decimal("a").unwrap(), 1);
        assert_eq!(params.get_decimal("b").unwrap(), 2);
        assert_eq!(params.get_decimal("c").unwrap(), 3);
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn add_b64_bytes() {
        let mut params = ParamsString::new();
        params.add_b64_bytes("a", &[1]).unwrap();
        params.add_b64_bytes("b", &[2, 3]).unwrap();
        params.add_b64_bytes("c", &[4, 5, 6]).unwrap();
        assert_eq!(params.to_string(), "a=AQ,b=AgM,c=BAUG");
    }

    #[test]
    fn duplicate_names() {
        let name = Ident::new("a").unwrap();
        let mut params = ParamsString::new();
        params.add_decimal(name, 1).unwrap();

        let err = params.add_decimal(name, 2u32.into()).err().unwrap();
        assert_eq!(err, Error::ParamNameDuplicated);
    }

    #[test]
    fn from_iter() {
        let params = ParamsString::from_iter(
            [
                (Ident::new("a").unwrap(), Value::try_from("1").unwrap()),
                (Ident::new("b").unwrap(), Value::try_from("2").unwrap()),
                (Ident::new("c").unwrap(), Value::try_from("3").unwrap()),
            ]
            .iter()
            .cloned(),
        );

        assert_eq!(params.iter().count(), 3);
        assert_eq!(params.get_decimal("a").unwrap(), 1);
        assert_eq!(params.get_decimal("b").unwrap(), 2);
        assert_eq!(params.get_decimal("c").unwrap(), 3);
    }

    #[test]
    fn iter() {
        let mut params = ParamsString::new();
        params.add_str("a", "1").unwrap();
        params.add_str("b", "2").unwrap();
        params.add_str("c", "3").unwrap();

        let mut i = params.iter();

        for (name, value) in &[("a", "1"), ("b", "2"), ("c", "3")] {
            let name = Ident::new(name).unwrap();
            let value = Value::try_from(*value).unwrap();
            assert_eq!(i.next(), Some((name, value)));
        }

        assert_eq!(i.next(), None);
    }

    //
    // `FromStr` tests
    //

    #[test]
    fn parse_empty() {
        let params = ParamsString::from_str("").unwrap();
        assert!(params.is_empty());
    }

    #[test]
    fn parse_one() {
        let params = ParamsString::from_str("a=1").unwrap();
        assert_eq!(params.iter().count(), 1);
        assert_eq!(params.get("a").unwrap().decimal().unwrap(), 1);
    }

    #[test]
    fn parse_many() {
        let params = ParamsString::from_str("a=1,b=2,c=3").unwrap();
        assert_eq!(params.iter().count(), 3);
        assert_eq!(params.get_decimal("a").unwrap(), 1);
        assert_eq!(params.get_decimal("b").unwrap(), 2);
        assert_eq!(params.get_decimal("c").unwrap(), 3);
    }

    //
    // `Display` tests
    //

    #[test]
    #[cfg(feature = "alloc")]
    fn display_empty() {
        let params = ParamsString::new();
        assert_eq!(params.to_string(), "");
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn display_one() {
        let params = ParamsString::from_str("a=1").unwrap();
        assert_eq!(params.to_string(), "a=1");
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn display_many() {
        let params = ParamsString::from_str("a=1,b=2,c=3").unwrap();
        assert_eq!(params.to_string(), "a=1,b=2,c=3");
    }
}
