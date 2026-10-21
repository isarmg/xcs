#[cfg(test)]
#[cfg(feature = "parse")]
#[cfg(feature = "display")]
mod test {
    use super::*;

    #[test]
    fn inline_tables() {
        let inputs = [
            r#"{}"#,
            r#"{   }"#,
            r#"{a = 1e165}"#,
            r#"{ hello = "world", a = 1}"#,
            r#"{ hello.world = "a" }"#,
        ];
        for input in inputs {
            dbg!(input);
            let mut parsed = inline_table.parse(new_input(input));
            if let Ok(parsed) = &mut parsed {
                parsed.despan(input);
            }
            assert_eq!(parsed.map(|a| a.to_string()), Ok(input.to_owned()));
        }
    }

    #[test]
    fn invalid_inline_tables() {
        let invalid_inputs = [r#"{a = 1e165"#, r#"{ hello = "world", a = 2, hello = 1}"#];
        for input in invalid_inputs {
            dbg!(input);
            let mut parsed = inline_table.parse(new_input(input));
            if let Ok(parsed) = &mut parsed {
                parsed.despan(input);
            }
            assert!(parsed.is_err());
        }
    }
}
