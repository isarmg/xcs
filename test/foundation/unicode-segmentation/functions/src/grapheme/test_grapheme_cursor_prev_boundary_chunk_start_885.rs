#[test]
fn test_grapheme_cursor_prev_boundary_chunk_start() {
    let s = "abcd";
    let mut c = GraphemeCursor::new(2, s.len(), true);
    assert_eq!(
        c.prev_boundary(&s[2..], 2),
        Err(GraphemeIncomplete::PrevChunk)
    );
    assert_eq!(c.prev_boundary(&s[..2], 0), Ok(Some(1)));
}
