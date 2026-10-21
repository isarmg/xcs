#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_matches {
        ($a:expr, $b:pat $(,)?) => {
            match $a {
                $b => {}
                a => panic!("`{:?}` doesn't match `{}`", a, stringify!($b)),
            }
        };
    }

    #[test]
    fn header() {
        assert!(Parser::default().parse(&[], true).is_err());
        assert_matches!(
            Parser::default().parse(&[], false),
            Ok(Chunk::NeedMoreData(4)),
        );
        assert_matches!(
            Parser::default().parse(b"\0", false),
            Ok(Chunk::NeedMoreData(3)),
        );
        assert_matches!(
            Parser::default().parse(b"\0asm", false),
            Ok(Chunk::NeedMoreData(4)),
        );
        assert_matches!(
            Parser::default().parse(b"\0asm\x01\0\0\0", false),
            Ok(Chunk::Parsed {
                consumed: 8,
                payload: Payload::Version { num: 1, .. },
            }),
        );
    }

    #[test]
    fn header_iter() {
        for _ in Parser::default().parse_all(&[]) {}
        for _ in Parser::default().parse_all(b"\0") {}
        for _ in Parser::default().parse_all(b"\0asm") {}
        for _ in Parser::default().parse_all(b"\0asm\x01\x01\x01\x01") {}
    }

    fn parser_after_header() -> Parser {
        let mut p = Parser::default();
        assert_matches!(
            p.parse(b"\0asm\x01\0\0\0", false),
            Ok(Chunk::Parsed {
                consumed: 8,
                payload: Payload::Version {
                    num: WASM_MODULE_VERSION,
                    encoding: Encoding::Module,
                    ..
                },
            }),
        );
        p
    }

    fn parser_after_component_header() -> Parser {
        let mut p = Parser::default();
        assert_matches!(
            p.parse(b"\0asm\x0d\0\x01\0", false),
            Ok(Chunk::Parsed {
                consumed: 8,
                payload: Payload::Version {
                    num: WASM_COMPONENT_VERSION,
                    encoding: Encoding::Component,
                    ..
                },
            }),
        );
        p
    }

    #[test]
    fn start_section() {
        assert_matches!(
            parser_after_header().parse(&[], false),
            Ok(Chunk::NeedMoreData(1)),
        );
        assert!(parser_after_header().parse(&[8], true).is_err());
        assert!(parser_after_header().parse(&[8, 1], true).is_err());
        assert!(parser_after_header().parse(&[8, 2], true).is_err());
        assert_matches!(
            parser_after_header().parse(&[8], false),
            Ok(Chunk::NeedMoreData(1)),
        );
        assert_matches!(
            parser_after_header().parse(&[8, 1], false),
            Ok(Chunk::NeedMoreData(1)),
        );
        assert_matches!(
            parser_after_header().parse(&[8, 2], false),
            Ok(Chunk::NeedMoreData(2)),
        );
        assert_matches!(
            parser_after_header().parse(&[8, 1, 1], false),
            Ok(Chunk::Parsed {
                consumed: 3,
                payload: Payload::StartSection { func: 1, .. },
            }),
        );
        assert!(parser_after_header().parse(&[8, 2, 1, 1], false).is_err());
        assert!(parser_after_header().parse(&[8, 0], false).is_err());
    }

    #[test]
    fn end_works() {
        assert_matches!(
            parser_after_header().parse(&[], true),
            Ok(Chunk::Parsed {
                consumed: 0,
                payload: Payload::End(8),
            }),
        );
    }

    #[test]
    fn type_section() {
        assert!(parser_after_header().parse(&[1], true).is_err());
        assert!(parser_after_header().parse(&[1, 0], false).is_err());
        assert!(parser_after_header().parse(&[8, 2], true).is_err());
        assert_matches!(
            parser_after_header().parse(&[1], false),
            Ok(Chunk::NeedMoreData(1)),
        );
        assert_matches!(
            parser_after_header().parse(&[1, 1], false),
            Ok(Chunk::NeedMoreData(1)),
        );
        assert_matches!(
            parser_after_header().parse(&[1, 1, 1], false),
            Ok(Chunk::Parsed {
                consumed: 3,
                payload: Payload::TypeSection(_),
            }),
        );
        assert_matches!(
            parser_after_header().parse(&[1, 1, 1, 2, 3, 4], false),
            Ok(Chunk::Parsed {
                consumed: 3,
                payload: Payload::TypeSection(_),
            }),
        );
    }

    #[test]
    fn custom_section() {
        assert!(parser_after_header().parse(&[0], true).is_err());
        assert!(parser_after_header().parse(&[0, 0], false).is_err());
        assert!(parser_after_header().parse(&[0, 1, 1], false).is_err());
        assert_matches!(
            parser_after_header().parse(&[0, 2, 1], false),
            Ok(Chunk::NeedMoreData(1)),
        );
        assert_custom(
            parser_after_header().parse(&[0, 1, 0], false).unwrap(),
            3,
            "",
            11,
            b"",
            Range { start: 10, end: 11 },
        );
        assert_custom(
            parser_after_header()
                .parse(&[0, 2, 1, b'a'], false)
                .unwrap(),
            4,
            "a",
            12,
            b"",
            Range { start: 10, end: 12 },
        );
        assert_custom(
            parser_after_header()
                .parse(&[0, 2, 0, b'a'], false)
                .unwrap(),
            4,
            "",
            11,
            b"a",
            Range { start: 10, end: 12 },
        );
    }

    fn assert_custom(
        chunk: Chunk<'_>,
        expected_consumed: usize,
        expected_name: &str,
        expected_data_offset: usize,
        expected_data: &[u8],
        expected_range: Range<usize>,
    ) {
        let (consumed, s) = match chunk {
            Chunk::Parsed {
                consumed,
                payload: Payload::CustomSection(s),
            } => (consumed, s),
            _ => panic!("not a custom section payload"),
        };
        assert_eq!(consumed, expected_consumed);
        assert_eq!(s.name(), expected_name);
        assert_eq!(s.data_offset(), expected_data_offset);
        assert_eq!(s.data(), expected_data);
        assert_eq!(s.range(), expected_range);
    }

    #[test]
    fn function_section() {
        assert!(parser_after_header().parse(&[10], true).is_err());
        assert!(parser_after_header().parse(&[10, 0], true).is_err());
        assert!(parser_after_header().parse(&[10, 1], true).is_err());
        assert_matches!(
            parser_after_header().parse(&[10], false),
            Ok(Chunk::NeedMoreData(1))
        );
        assert_matches!(
            parser_after_header().parse(&[10, 1], false),
            Ok(Chunk::NeedMoreData(1))
        );
        let mut p = parser_after_header();
        assert_matches!(
            p.parse(&[10, 1, 0], false),
            Ok(Chunk::Parsed {
                consumed: 3,
                payload: Payload::CodeSectionStart { count: 0, .. },
            }),
        );
        assert_matches!(
            p.parse(&[], true),
            Ok(Chunk::Parsed {
                consumed: 0,
                payload: Payload::End(11),
            }),
        );
        let mut p = parser_after_header();
        assert_matches!(
            p.parse(&[3, 2, 1, 0], false),
            Ok(Chunk::Parsed {
                consumed: 4,
                payload: Payload::FunctionSection { .. },
            }),
        );
        assert_matches!(
            p.parse(&[10, 2, 1, 0], false),
            Ok(Chunk::Parsed {
                consumed: 3,
                payload: Payload::CodeSectionStart { count: 1, .. },
            }),
        );
        assert_matches!(
            p.parse(&[0], false),
            Ok(Chunk::Parsed {
                consumed: 1,
                payload: Payload::CodeSectionEntry(_),
            }),
        );
        assert_matches!(
            p.parse(&[], true),
            Ok(Chunk::Parsed {
                consumed: 0,
                payload: Payload::End(16),
            }),
        );

        // 1 byte section with 1 function can't read the function body because
        // the section is too small
        let mut p = parser_after_header();
        assert_matches!(
            p.parse(&[3, 2, 1, 0], false),
            Ok(Chunk::Parsed {
                consumed: 4,
                payload: Payload::FunctionSection { .. },
            }),
        );
        assert_matches!(
            p.parse(&[10, 1, 1], false),
            Ok(Chunk::Parsed {
                consumed: 3,
                payload: Payload::CodeSectionStart { count: 1, .. },
            }),
        );
        assert_eq!(
            p.parse(&[0], false).unwrap_err().message(),
            "unexpected end-of-file"
        );

        // section with 2 functions but section is cut off
        let mut p = parser_after_header();
        assert_matches!(
            p.parse(&[3, 2, 2, 0], false),
            Ok(Chunk::Parsed {
                consumed: 4,
                payload: Payload::FunctionSection { .. },
            }),
        );
        assert_matches!(
            p.parse(&[10, 2, 2], false),
            Ok(Chunk::Parsed {
                consumed: 3,
                payload: Payload::CodeSectionStart { count: 2, .. },
            }),
        );
        assert_matches!(
            p.parse(&[0], false),
            Ok(Chunk::Parsed {
                consumed: 1,
                payload: Payload::CodeSectionEntry(_),
            }),
        );
        assert_matches!(p.parse(&[], false), Ok(Chunk::NeedMoreData(1)));
        assert_eq!(
            p.parse(&[0], false).unwrap_err().message(),
            "unexpected end-of-file",
        );

        // trailing data is bad
        let mut p = parser_after_header();
        assert_matches!(
            p.parse(&[3, 2, 1, 0], false),
            Ok(Chunk::Parsed {
                consumed: 4,
                payload: Payload::FunctionSection { .. },
            }),
        );
        assert_matches!(
            p.parse(&[10, 3, 1], false),
            Ok(Chunk::Parsed {
                consumed: 3,
                payload: Payload::CodeSectionStart { count: 1, .. },
            }),
        );
        assert_matches!(
            p.parse(&[0], false),
            Ok(Chunk::Parsed {
                consumed: 1,
                payload: Payload::CodeSectionEntry(_),
            }),
        );
        assert_eq!(
            p.parse(&[0], false).unwrap_err().message(),
            "trailing bytes at end of section",
        );
    }

    #[test]
    fn single_module() {
        let mut p = parser_after_component_header();
        assert_matches!(p.parse(&[4], false), Ok(Chunk::NeedMoreData(1)));

        // A module that's 8 bytes in length
        let mut sub = match p.parse(&[1, 8], false) {
            Ok(Chunk::Parsed {
                consumed: 2,
                payload: Payload::ModuleSection { parser, .. },
            }) => parser,
            other => panic!("bad parse {other:?}"),
        };

        // Parse the header of the submodule with the sub-parser.
        assert_matches!(sub.parse(&[], false), Ok(Chunk::NeedMoreData(4)));
        assert_matches!(sub.parse(b"\0asm", false), Ok(Chunk::NeedMoreData(4)));
        assert_matches!(
            sub.parse(b"\0asm\x01\0\0\0", false),
            Ok(Chunk::Parsed {
                consumed: 8,
                payload: Payload::Version {
                    num: 1,
                    encoding: Encoding::Module,
                    ..
                },
            }),
        );

        // The sub-parser should be byte-limited so the next byte shouldn't get
        // consumed, it's intended for the parent parser.
        assert_matches!(
            sub.parse(&[10], false),
            Ok(Chunk::Parsed {
                consumed: 0,
                payload: Payload::End(18),
            }),
        );

        // The parent parser should now be back to resuming, and we simulate it
        // being done with bytes to ensure that it's safely at the end,
        // completing the module code section.
        assert_matches!(p.parse(&[], false), Ok(Chunk::NeedMoreData(1)));
        assert_matches!(
            p.parse(&[], true),
            Ok(Chunk::Parsed {
                consumed: 0,
                payload: Payload::End(18),
            }),
        );
    }

    #[test]
    fn nested_section_too_big() {
        let mut p = parser_after_component_header();

        // A module that's 10 bytes in length
        let mut sub = match p.parse(&[1, 10], false) {
            Ok(Chunk::Parsed {
                consumed: 2,
                payload: Payload::ModuleSection { parser, .. },
            }) => parser,
            other => panic!("bad parse {other:?}"),
        };

        // use 8 bytes to parse the header, leaving 2 remaining bytes in our
        // module.
        assert_matches!(
            sub.parse(b"\0asm\x01\0\0\0", false),
            Ok(Chunk::Parsed {
                consumed: 8,
                payload: Payload::Version { num: 1, .. },
            }),
        );

        // We can't parse a section which declares its bigger than the outer
        // module. This is a custom section, one byte big, with one content byte. The
        // content byte, however, lives outside of the parent's module code
        // section.
        assert_eq!(
            sub.parse(&[0, 1, 0], false).unwrap_err().message(),
            "section too large",
        );
    }
}
