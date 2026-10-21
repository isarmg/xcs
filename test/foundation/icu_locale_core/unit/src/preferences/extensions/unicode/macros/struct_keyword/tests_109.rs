#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        extensions::unicode,
        subtags::{Subtag, subtag},
    };
    use core::str::FromStr;

    #[test]
    fn struct_keywords_test() {
        struct_keyword!(
            DummyKeyword,
            "dk",
            Subtag,
            |input: &unicode::Value| {
                if let Some(&subtag) = input.as_single_subtag()
                    && subtag.len() == 3
                {
                    return Ok(DummyKeyword(subtag));
                }
                Err(crate::preferences::extensions::unicode::errors::PreferencesParseError::InvalidKeywordValue)
            },
            |input: &DummyKeyword| { unicode::Value::from_subtag(Some(input.0)) }
        );

        let v = unicode::Value::from_str("foo").unwrap();
        let dk: DummyKeyword = v.clone().try_into().unwrap();
        assert_eq!(dk, DummyKeyword(subtag!("foo")));
        assert_eq!(unicode::Value::from(dk), v);

        let v = unicode::Value::from_str("foobar").unwrap();
        let dk: Result<DummyKeyword, _> = v.clone().try_into();
        assert!(dk.is_err());
    }
}
