
#[cfg(test)]
mod tests {
    use attribute_derive::FromAttr;
    use std::io::Write;

    #[derive(Debug, FromAttr)]
    #[from_attr(ident = one)]
    struct One { required: String }

    #[derive(Debug, FromAttr)]
    struct Two { alpha: Option<String>, beta: Option<String> }

    #[derive(Debug, FromAttr)]
    struct Flags {
        #[from_attr(conflicts = [beta])]
        alpha: bool,
        beta: bool,
    }

    #[derive(FromAttr)]
    struct Expression { expression: rust_syntax::ExprIf }

    #[test]
    fn full_syntax_attributes_parse_expression_nodes() {
        let parsed = rust_syntax::parse_str::<Expression>(
            "expression = if true { 1 } else { 2 }"
        ).unwrap();
        assert!(parsed.expression.else_branch.is_some());
    }

    #[test]
    fn attribute_templates_preserve_missing_duplicate_unknown_and_conflict_diagnostics() {
        assert_eq!(rust_syntax::parse_str::<One>(r#"required = "文件 Ω""#).unwrap().required, "文件 Ω");
        let unknown = rust_syntax::parse_str::<One>("unknown = 1").unwrap_err().to_string();
        assert_eq!(unknown, "expected supported field `required`");
        let missing = rust_syntax::parse_str::<One>("").unwrap_err().to_string();
        assert_eq!(missing, "required `required` is not specified\n\n= help: try `#[one(required = ...)]`");
        let duplicate = rust_syntax::parse_str::<One>(r#"required = "a", required = "b""#).unwrap_err().to_string();
        assert_eq!(duplicate, "`required` is specified multiple times");
        let two = rust_syntax::parse_str::<Two>("").unwrap();
        assert!(two.alpha.is_none() && two.beta.is_none());
        assert_eq!(rust_syntax::parse_str::<Two>("unknown = 1").unwrap_err().to_string(),
                   "supported fields are `alpha` and `beta`");
        let flags = rust_syntax::parse_str::<Flags>("alpha").unwrap();
        assert!(flags.alpha && !flags.beta);
        let conflict = rust_syntax::parse_str::<Flags>("alpha, beta").unwrap_err().to_string();
        assert!(conflict.contains("conflicts with mutually exclusive"), "{conflict}");
        assert!(conflict.contains("alpha") && conflict.contains("beta"), "{conflict}");
    }

    #[test]
    fn parser_keeps_unicode_partial_input_and_binary_contracts_without_tracing() {
        use winnow::{Parser, ModalResult, error::{ContextError, ErrMode, Needed},
                     stream::Partial, token::{literal, take_while}};
        fn assignment<'i>(input: &mut &'i str) -> ModalResult<&'i str> {
            ("name=", take_while(1.., |c: char| c != ';'), ';')
                .map(|(_, name, _)| name).parse_next(input)
        }
        assert_eq!(assignment.parse("name=文件 Ω;").unwrap(), "文件 Ω");
        assert!(assignment.parse("name=;").is_err());
        let mut partial = Partial::new("ab");
        let result: Result<_, ErrMode<ContextError>> = literal("abc").parse_next(&mut partial);
        assert_eq!(result, Err(ErrMode::Incomplete(Needed::Unknown)));
        assert_eq!(*partial, "ab");
        let mut partial_bytes = Partial::new(&b"ab"[..]);
        let result: Result<_, ErrMode<ContextError>> = literal(&b"abc"[..]).parse_next(&mut partial_bytes);
        assert_eq!(result, Err(ErrMode::Incomplete(Needed::Unknown)));
        assert_eq!(*partial_bytes, &b"ab"[..]);
        let mut empty = Partial::new(&b""[..]);
        let result: Result<u8, ErrMode<ContextError>> = winnow::token::any.parse_next(&mut empty);
        assert_eq!(result, Err(ErrMode::Incomplete(Needed::new(1))));
        let result: ModalResult<u16> = winnow::binary::be_u16.parse_next(&mut &[0x12, 0x34][..]);
        assert_eq!(result.unwrap(), 0x1234);
        let source = "title = \"文件 Ω\"\nvalue = 1_024\n";
        let document: toml::editor::DocumentMut = source.parse().unwrap();
        assert_eq!(document["title"].as_str(), Some("文件 Ω"));
        assert_eq!(document["value"].as_integer(), Some(1024));
        assert_eq!(document.to_string(), source);
    }

    #[test]
    fn terminal_output_preserves_unicode_and_explicit_color_policy() {
        let bytes = "\x1b[31m文件 Ω\x1b[0m\n".as_bytes();
        let mut plain = terminal_style::output::AutoStream::new(Vec::new(), terminal_style::ColorChoice::Never);
        plain.write_all(bytes).unwrap();
        assert_eq!(plain.into_inner(), "文件 Ω\n".as_bytes());
        let mut color = terminal_style::output::AutoStream::new(Vec::new(), terminal_style::ColorChoice::Always);
        color.write_all(bytes).unwrap();
        assert_eq!(color.into_inner(), bytes);
        let file = std::fs::File::open("/dev/null").unwrap();
        assert!(!terminal_style::output::stream::IsTerminal::is_terminal(&file));
    }

    #[test]
    fn clang_arguments_and_bitfield_runs_preserve_neighboring_fields() {
        // The invoking script supplies a shell-quoted definition containing a space.
        let source = r#"
            #if VALUE != 7
            #error expected external clang arguments
            #endif
            static const char *label = LABEL;
            struct Packet {
                int leading;
                unsigned low : 3;
                unsigned high : 5;
                int middle;
                unsigned last : 2;
                int trailing;
            };
        "#;
        let formatter: bindgen::Formatter = "rust_source_format".parse().unwrap();
        assert_eq!(formatter, bindgen::Formatter::RustSourceFormat);
        assert_eq!(formatter.to_string(), "rust_source_format");
        let output = bindgen::Builder::default()
            .formatter(formatter)
            .header_contents("foundation-consumer.h", source)
            .layout_tests(false)
            .generate().expect("actual C parsing and binding generation").to_string();
        for field in ["pub leading:", "pub middle:", "pub trailing:", "fn low(", "fn high(", "fn last("] {
            assert!(output.contains(field), "missing {field}: {output}");
        }
    }
}
