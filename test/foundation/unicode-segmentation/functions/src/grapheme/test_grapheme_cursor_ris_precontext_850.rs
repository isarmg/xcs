#[test]
fn test_grapheme_cursor_ris_precontext() {
    let s = "\u{1f1fa}\u{1f1f8}\u{1f1fa}\u{1f1f8}\u{1f1fa}\u{1f1f8}";
    let mut c = GraphemeCursor::new(8, s.len(), true);
    assert_eq!(
        c.is_boundary(&s[4..], 4),
        Err(GraphemeIncomplete::PreContext(4))
    );
    c.provide_context(&s[..4], 0);
    assert_eq!(c.is_boundary(&s[4..], 4), Ok(true));
}
