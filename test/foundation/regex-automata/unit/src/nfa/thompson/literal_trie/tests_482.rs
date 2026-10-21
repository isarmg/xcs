#[cfg(test)]
mod tests {
    use bstr::B;
    use regex_syntax::hir::Hir;

    use super::*;

    #[test]
    fn zap() {
        let mut trie = LiteralTrie::forward();
        trie.add(b"zapper").unwrap();
        trie.add(b"z").unwrap();
        trie.add(b"zap").unwrap();

        let got = trie.compile_to_hir();
        let expected = Hir::concat(vec![
            Hir::literal(B("z")),
            Hir::alternation(vec![
                Hir::literal(B("apper")),
                Hir::empty(),
                Hir::literal(B("ap")),
            ]),
        ]);
        assert_eq!(expected, got);
    }

    #[test]
    fn maker() {
        let mut trie = LiteralTrie::forward();
        trie.add(b"make").unwrap();
        trie.add(b"maple").unwrap();
        trie.add(b"maker").unwrap();

        let got = trie.compile_to_hir();
        let expected = Hir::concat(vec![
            Hir::literal(B("ma")),
            Hir::alternation(vec![
                Hir::concat(vec![
                    Hir::literal(B("ke")),
                    Hir::alternation(vec![Hir::empty(), Hir::literal(B("r"))]),
                ]),
                Hir::literal(B("ple")),
            ]),
        ]);
        assert_eq!(expected, got);
    }
}
