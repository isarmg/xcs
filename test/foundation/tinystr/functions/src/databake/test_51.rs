#[test]
fn test() {
    test_bake!(
        TinyAsciiStr<10>,
        const,
        crate::tinystr!(10usize, "foo"),
        tinystr
    );
}
