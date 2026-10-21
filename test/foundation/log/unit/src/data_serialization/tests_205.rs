#[cfg(test)]
mod tests {
    use crate::{Level, LevelFilter};
    use serde_test::{assert_de_tokens, assert_de_tokens_error, assert_tokens, Token};

    fn level_token(variant: &'static str) -> Token {
        Token::UnitVariant {
            name: "Level",
            variant,
        }
    }

    fn level_bytes_tokens(variant: &'static [u8]) -> [Token; 3] {
        [
            Token::Enum { name: "Level" },
            Token::Bytes(variant),
            Token::Unit,
        ]
    }

    fn level_variant_tokens(variant: u32) -> [Token; 3] {
        [
            Token::Enum { name: "Level" },
            Token::U32(variant),
            Token::Unit,
        ]
    }

    fn level_filter_token(variant: &'static str) -> Token {
        Token::UnitVariant {
            name: "LevelFilter",
            variant,
        }
    }

    fn level_filter_bytes_tokens(variant: &'static [u8]) -> [Token; 3] {
        [
            Token::Enum {
                name: "LevelFilter",
            },
            Token::Bytes(variant),
            Token::Unit,
        ]
    }

    fn level_filter_variant_tokens(variant: u32) -> [Token; 3] {
        [
            Token::Enum {
                name: "LevelFilter",
            },
            Token::U32(variant),
            Token::Unit,
        ]
    }

    #[test]
    fn test_level_ser_de() {
        let cases = &[
            (Level::Error, [level_token("ERROR")]),
            (Level::Warn, [level_token("WARN")]),
            (Level::Info, [level_token("INFO")]),
            (Level::Debug, [level_token("DEBUG")]),
            (Level::Trace, [level_token("TRACE")]),
        ];

        for (s, expected) in cases {
            assert_tokens(s, expected);
        }
    }

    #[test]
    fn test_level_case_insensitive() {
        let cases = &[
            (Level::Error, [level_token("error")]),
            (Level::Warn, [level_token("warn")]),
            (Level::Info, [level_token("info")]),
            (Level::Debug, [level_token("debug")]),
            (Level::Trace, [level_token("trace")]),
        ];

        for (s, expected) in cases {
            assert_de_tokens(s, expected);
        }
    }

    #[test]
    fn test_level_de_bytes() {
        let cases = &[
            (Level::Error, level_bytes_tokens(b"ERROR")),
            (Level::Warn, level_bytes_tokens(b"WARN")),
            (Level::Info, level_bytes_tokens(b"INFO")),
            (Level::Debug, level_bytes_tokens(b"DEBUG")),
            (Level::Trace, level_bytes_tokens(b"TRACE")),
        ];

        for (value, tokens) in cases {
            assert_de_tokens(value, tokens);
        }
    }

    #[test]
    fn test_level_de_variant_index() {
        let cases = &[
            (Level::Error, level_variant_tokens(0)),
            (Level::Warn, level_variant_tokens(1)),
            (Level::Info, level_variant_tokens(2)),
            (Level::Debug, level_variant_tokens(3)),
            (Level::Trace, level_variant_tokens(4)),
        ];

        for (value, tokens) in cases {
            assert_de_tokens(value, tokens);
        }
    }

    #[test]
    fn test_level_de_error() {
        let msg = "unknown variant `errorx`, expected one of \
                   `ERROR`, `WARN`, `INFO`, `DEBUG`, `TRACE`";
        assert_de_tokens_error::<Level>(&[level_token("errorx")], msg);
    }

    #[test]
    fn test_level_filter_ser_de() {
        let cases = &[
            (LevelFilter::Off, [level_filter_token("OFF")]),
            (LevelFilter::Error, [level_filter_token("ERROR")]),
            (LevelFilter::Warn, [level_filter_token("WARN")]),
            (LevelFilter::Info, [level_filter_token("INFO")]),
            (LevelFilter::Debug, [level_filter_token("DEBUG")]),
            (LevelFilter::Trace, [level_filter_token("TRACE")]),
        ];

        for (s, expected) in cases {
            assert_tokens(s, expected);
        }
    }

    #[test]
    fn test_level_filter_case_insensitive() {
        let cases = &[
            (LevelFilter::Off, [level_filter_token("off")]),
            (LevelFilter::Error, [level_filter_token("error")]),
            (LevelFilter::Warn, [level_filter_token("warn")]),
            (LevelFilter::Info, [level_filter_token("info")]),
            (LevelFilter::Debug, [level_filter_token("debug")]),
            (LevelFilter::Trace, [level_filter_token("trace")]),
        ];

        for (s, expected) in cases {
            assert_de_tokens(s, expected);
        }
    }

    #[test]
    fn test_level_filter_de_bytes() {
        let cases = &[
            (LevelFilter::Off, level_filter_bytes_tokens(b"OFF")),
            (LevelFilter::Error, level_filter_bytes_tokens(b"ERROR")),
            (LevelFilter::Warn, level_filter_bytes_tokens(b"WARN")),
            (LevelFilter::Info, level_filter_bytes_tokens(b"INFO")),
            (LevelFilter::Debug, level_filter_bytes_tokens(b"DEBUG")),
            (LevelFilter::Trace, level_filter_bytes_tokens(b"TRACE")),
        ];

        for (value, tokens) in cases {
            assert_de_tokens(value, tokens);
        }
    }

    #[test]
    fn test_level_filter_de_variant_index() {
        let cases = &[
            (LevelFilter::Off, level_filter_variant_tokens(0)),
            (LevelFilter::Error, level_filter_variant_tokens(1)),
            (LevelFilter::Warn, level_filter_variant_tokens(2)),
            (LevelFilter::Info, level_filter_variant_tokens(3)),
            (LevelFilter::Debug, level_filter_variant_tokens(4)),
            (LevelFilter::Trace, level_filter_variant_tokens(5)),
        ];

        for (value, tokens) in cases {
            assert_de_tokens(value, tokens);
        }
    }

    #[test]
    fn test_level_filter_de_error() {
        let msg = "unknown variant `errorx`, expected one of \
                   `OFF`, `ERROR`, `WARN`, `INFO`, `DEBUG`, `TRACE`";
        assert_de_tokens_error::<LevelFilter>(&[level_filter_token("errorx")], msg);
    }
}
