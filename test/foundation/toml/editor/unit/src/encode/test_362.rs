#[cfg(any())] // Original randomized/shrinking tests retained; no proptest provider.
mod test {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        #[cfg(feature = "parse")]
        fn parseable_string(string in "\\PC*") {
            let value = Value::from(string.clone());
            let encoded = value.to_string();
            let _: Value = encoded.parse().unwrap_or_else(|err| {
                panic!("error: {err}

string:
```
{string}
```
value:
```
{value}
```
")
            });
        }
    }

    proptest! {
        #[test]
        #[cfg(feature = "parse")]
        fn parseable_key(string in "\\PC*") {
            let key = Key::new(string.clone());
            let encoded = key.to_string();
            let _: Key = encoded.parse().unwrap_or_else(|err| {
                panic!("error: {err}

string:
```
{string}
```
key:
```
{key}
```
")
            });
        }
    }
}

// Deterministic coverage, not an emulation of the retained proptest engine.
#[cfg(all(test, feature = "parse"))]
mod unicode_regressions {
    use super::*;
    #[test]
    fn all_unicode_scalars_roundtrip() {
        for scalar in 0..=0x10ffff {
            let Some(character) = char::from_u32(scalar) else { continue };
            let text = character.to_string();
            let parsed: Value = Value::from(text.clone()).to_string().parse().unwrap();
            assert_eq!(parsed.as_str(), Some(text.as_str()));
            let parsed: Key = Key::new(text.clone()).to_string().parse().unwrap();
            assert_eq!(parsed.get(), text);
        }
    }
    #[test]
    fn string_boundaries_roundtrip() {
        for text in ["", "\r\n", "\\\"'", "a\n\nb", "中文🦀", "\0\u{7f}\u{85}", "''''", "\"\"\"", "a\t b", "x\r\ny"] {
            let parsed: Value = Value::from(text).to_string().parse().unwrap();
            assert_eq!(parsed.as_str(), Some(text));
            let parsed: Key = Key::new(text).to_string().parse().unwrap();
            assert_eq!(parsed.get(), text);
        }
    }
}
