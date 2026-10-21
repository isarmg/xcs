#[test]
fn test_grapheme_cursor_chunk_start_require_precontext() {
    let s = "\r\n";
    let mut c = GraphemeCursor::new(1, s.len(), true);
    assert_eq!(
        c.is_boundary(&s[1..], 1),
        Err(GraphemeIncomplete::PreContext(1))
    );
    c.provide_context(&s[..1], 0);
    assert_eq!(c.is_boundary(&s[1..], 1), Ok(false));
}
