#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn picks_up_single_and_multi_line_doc_comments() {
        assert_eq!(kind("/// hello"), Some(Kind::SingleLines));
        assert_eq!(kind("/** world */"), Some(Kind::MultiLine));
    }

    #[test]
    fn processes_single_lines_correctly() {
        assert_eq!(preprocess("///"), "");
        assert_eq!(preprocess("/// hello"), " hello");
        assert_eq!(preprocess("// hello"), " hello");
        assert_eq!(preprocess("//    hello"), "    hello");
    }

    #[test]
    fn processes_multi_lines_correctly() {
        assert_eq!(preprocess("/**/"), "");

        assert_eq!(
            preprocess("/** hello \n * world \n * foo \n */"),
            " hello\n world\n foo"
        );

        assert_eq!(
            preprocess("/**\nhello\n*world\n*foo\n*/"),
            "hello\nworld\nfoo"
        );
    }
}
