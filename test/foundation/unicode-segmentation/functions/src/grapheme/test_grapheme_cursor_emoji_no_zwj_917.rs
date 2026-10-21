#[test]
fn test_grapheme_cursor_emoji_no_zwj() {
    use GraphemeIncomplete::*;
    let chunk0 = "🍒"; // 4 bytes
    let chunk1 = "🥑"; // 4 bytes
    let full_len = chunk0.len() + chunk1.len();

    let mut c = GraphemeCursor::new(0, full_len, true);
    assert_eq!(c.next_boundary(chunk0, 0), Err(NextChunk));
    assert_eq!(
        c.next_boundary(chunk1, chunk0.len()),
        Err(PreContext(chunk0.len()))
    );
    c.provide_context(chunk0, 0);
    assert_eq!(c.next_boundary(chunk1, chunk0.len()), Ok(Some(4)));
    assert_eq!(c.next_boundary(chunk1, chunk0.len()), Ok(Some(8)));
    assert_eq!(c.next_boundary(chunk1, chunk0.len()), Ok(None));
}
