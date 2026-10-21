#[cfg(test)]
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
