#[cfg(test)]
mod tests {
    use alloc::string::String;

    use crate::syntax::ast::parse::ParserBuilder;

    use super::*;

    fn roundtrip(given: &str) {
        roundtrip_with(|b| b, given);
    }

    fn roundtrip_with<F>(mut f: F, given: &str)
    where
        F: FnMut(&mut ParserBuilder) -> &mut ParserBuilder,
    {
        let mut builder = ParserBuilder::new();
        f(&mut builder);
        let ast = builder.build().parse(given).unwrap();

        let mut printer = Printer::new();
        let mut dst = String::new();
        printer.print(&ast, &mut dst).unwrap();
        assert_eq!(given, dst);
    }

    #[test]
    fn print_literal() {
        roundtrip("a");
        roundtrip(r"\[");
        roundtrip_with(|b| b.octal(true), r"\141");
        roundtrip(r"\x61");
        roundtrip(r"\x7F");
        roundtrip(r"\u0061");
        roundtrip(r"\U00000061");
        roundtrip(r"\x{61}");
        roundtrip(r"\x{7F}");
        roundtrip(r"\u{61}");
        roundtrip(r"\U{61}");

        roundtrip(r"\a");
        roundtrip(r"\f");
        roundtrip(r"\t");
        roundtrip(r"\n");
        roundtrip(r"\r");
        roundtrip(r"\v");
        roundtrip(r"(?x)\ ");
    }

    #[test]
    fn print_dot() {
        roundtrip(".");
    }

    #[test]
    fn print_concat() {
        roundtrip("ab");
        roundtrip("abcde");
        roundtrip("a(bcd)ef");
    }

    #[test]
    fn print_alternation() {
        roundtrip("a|b");
        roundtrip("a|b|c|d|e");
        roundtrip("|a|b|c|d|e");
        roundtrip("|a|b|c|d|e|");
        roundtrip("a(b|c|d)|e|f");
    }

    #[test]
    fn print_assertion() {
        roundtrip(r"^");
        roundtrip(r"$");
        roundtrip(r"\A");
        roundtrip(r"\z");
        roundtrip(r"\b");
        roundtrip(r"\B");
    }

    #[test]
    fn print_repetition() {
        roundtrip("a?");
        roundtrip("a??");
        roundtrip("a*");
        roundtrip("a*?");
        roundtrip("a+");
        roundtrip("a+?");
        roundtrip("a{5}");
        roundtrip("a{5}?");
        roundtrip("a{5,}");
        roundtrip("a{5,}?");
        roundtrip("a{5,10}");
        roundtrip("a{5,10}?");
    }

    #[test]
    fn print_flags() {
        roundtrip("(?i)");
        roundtrip("(?-i)");
        roundtrip("(?s-i)");
        roundtrip("(?-si)");
        roundtrip("(?siUmux)");
    }

    #[test]
    fn print_group() {
        roundtrip("(?i:a)");
        roundtrip("(?P<foo>a)");
        roundtrip("(?<foo>a)");
        roundtrip("(a)");
    }

    #[test]
    fn print_class() {
        roundtrip(r"[abc]");
        roundtrip(r"[a-z]");
        roundtrip(r"[^a-z]");
        roundtrip(r"[a-z0-9]");
        roundtrip(r"[-a-z0-9]");
        roundtrip(r"[-a-z0-9]");
        roundtrip(r"[a-z0-9---]");
        roundtrip(r"[a-z&&m-n]");
        roundtrip(r"[[a-z&&m-n]]");
        roundtrip(r"[a-z--m-n]");
        roundtrip(r"[a-z~~m-n]");
        roundtrip(r"[a-z[0-9]]");
        roundtrip(r"[a-z[^0-9]]");

        roundtrip(r"\d");
        roundtrip(r"\D");
        roundtrip(r"\s");
        roundtrip(r"\S");
        roundtrip(r"\w");
        roundtrip(r"\W");

        roundtrip(r"[[:alnum:]]");
        roundtrip(r"[[:^alnum:]]");
        roundtrip(r"[[:alpha:]]");
        roundtrip(r"[[:^alpha:]]");
        roundtrip(r"[[:ascii:]]");
        roundtrip(r"[[:^ascii:]]");
        roundtrip(r"[[:blank:]]");
        roundtrip(r"[[:^blank:]]");
        roundtrip(r"[[:cntrl:]]");
        roundtrip(r"[[:^cntrl:]]");
        roundtrip(r"[[:digit:]]");
        roundtrip(r"[[:^digit:]]");
        roundtrip(r"[[:graph:]]");
        roundtrip(r"[[:^graph:]]");
        roundtrip(r"[[:lower:]]");
        roundtrip(r"[[:^lower:]]");
        roundtrip(r"[[:print:]]");
        roundtrip(r"[[:^print:]]");
        roundtrip(r"[[:punct:]]");
        roundtrip(r"[[:^punct:]]");
        roundtrip(r"[[:space:]]");
        roundtrip(r"[[:^space:]]");
        roundtrip(r"[[:upper:]]");
        roundtrip(r"[[:^upper:]]");
        roundtrip(r"[[:word:]]");
        roundtrip(r"[[:^word:]]");
        roundtrip(r"[[:xdigit:]]");
        roundtrip(r"[[:^xdigit:]]");

        roundtrip(r"\pL");
        roundtrip(r"\PL");
        roundtrip(r"\p{L}");
        roundtrip(r"\P{L}");
        roundtrip(r"\p{X=Y}");
        roundtrip(r"\P{X=Y}");
        roundtrip(r"\p{X:Y}");
        roundtrip(r"\P{X:Y}");
        roundtrip(r"\p{X!=Y}");
        roundtrip(r"\P{X!=Y}");
    }
}
