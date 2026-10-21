#[cfg(test)]
mod tests {
    use super::*;
    use crate::extensions::unicode;
    use core::str::FromStr;

    #[test]
    fn enum_keywords_test() {
        enum_keyword!(DummyKeyword {
            ("standard" => Standard),
            ("rare" => Rare),
        }, "dk");

        let v = unicode::Value::from_str("standard").unwrap();
        let dk = DummyKeyword::try_from(&v).unwrap();
        assert_eq!(dk, DummyKeyword::Standard);
        assert_eq!(unicode::Value::from(dk), v);

        let v = unicode::Value::from_str("rare").unwrap();
        let dk = DummyKeyword::try_from(&v).unwrap();
        assert_eq!(dk, DummyKeyword::Rare);
        assert_eq!(unicode::Value::from(dk), v);

        let v = unicode::Value::from_str("foo").unwrap();
        let dk = DummyKeyword::try_from(&v);
        dk.unwrap_err();

        assert_eq!(DummyKeyword::Standard.as_str(), "standard");
    }

    #[test]
    fn enum_keywords_test_alias() {
        enum_keyword!(DummyKeyword {
            ("standard" => Standard),
            ("rare" => Rare),
        }, "dk", s, if *s == value!("std") { return Ok(Self::Standard) });

        let v = unicode::Value::from_str("standard").unwrap();
        let dk = DummyKeyword::try_from(&v).unwrap();
        assert_eq!(dk, DummyKeyword::Standard);
        assert_eq!(unicode::Value::from(dk), v);

        let v_alias = unicode::Value::from_str("std").unwrap();
        let dk = DummyKeyword::try_from(&v_alias).unwrap();
        assert_eq!(dk, DummyKeyword::Standard);
        assert_eq!(unicode::Value::from(dk), v);

        let v = unicode::Value::from_str("rare").unwrap();
        let dk = DummyKeyword::try_from(&v).unwrap();
        assert_eq!(dk, DummyKeyword::Rare);
        assert_eq!(unicode::Value::from(dk), v);

        let v = unicode::Value::from_str("foo").unwrap();
        let dk = DummyKeyword::try_from(&v);
        dk.unwrap_err();

        assert_eq!(DummyKeyword::Standard.as_str(), "standard");
    }

    #[test]
    fn enum_keywords_nested_test() {
        enum_keyword!(DummySubKeyword { Standard, Rare });

        enum_keyword!(DummyKeyword {
            ("default" => Default),
            ("sub" => Sub(DummySubKeyword) {
                ("standard" => Standard),
                ("rare" => Rare)
            })
        }, "dk");

        let v = unicode::Value::from_str("default").unwrap();
        let dk = DummyKeyword::try_from(&v).unwrap();
        assert_eq!(dk, DummyKeyword::Default);
        assert_eq!(unicode::Value::from(dk), v);

        let v = unicode::Value::from_str("sub").unwrap();
        let dk = DummyKeyword::try_from(&v).unwrap();
        assert_eq!(dk, DummyKeyword::Sub(None));
        assert_eq!(unicode::Value::from(dk), v);

        let v = unicode::Value::from_str("foo").unwrap();
        let dk = DummyKeyword::try_from(&v);
        dk.unwrap_err();

        let v = unicode::Value::from_str("sub-standard").unwrap();
        let dk = DummyKeyword::try_from(&v).unwrap();
        assert_eq!(dk, DummyKeyword::Sub(Some(DummySubKeyword::Standard)));
        assert_eq!(unicode::Value::from(dk), v);

        let v = unicode::Value::from_str("sub-rare").unwrap();
        let dk = DummyKeyword::try_from(&v).unwrap();
        assert_eq!(dk, DummyKeyword::Sub(Some(DummySubKeyword::Rare)));
        assert_eq!(unicode::Value::from(dk), v);

        let v = unicode::Value::from_str("sub-foo").unwrap();
        let dk = DummyKeyword::try_from(&v);
        dk.unwrap_err();

        assert_eq!(
            DummyKeyword::Sub(Some(DummySubKeyword::Rare)).as_str(),
            "sub-rare"
        );
    }
}
