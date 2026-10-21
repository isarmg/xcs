#[test]
fn test_grapheme_cursor_emoji_zwj_across_chunks() {
    use GraphemeIncomplete::*;
    let chunk0 = "🍒"; // 4 bytes
    let chunk1 = "\u{200d}"; // 3 bytes
    let chunk2 = "🥑"; // 4 bytes
    let full_len = chunk0.len() + chunk1.len() + chunk2.len(); // 11
    let chunk2_start = chunk0.len() + chunk1.len();

    let mut c = GraphemeCursor::new(0, full_len, true);
    assert_eq!(c.next_boundary(chunk0, 0), Err(NextChunk));
    assert_eq!(c.next_boundary(chunk1, chunk0.len()), Err(NextChunk));
    assert_eq!(
        c.next_boundary(chunk2, chunk2_start),
        Err(PreContext(chunk2_start))
    );
    c.provide_context(chunk1, chunk0.len());
    assert_eq!(
        c.next_boundary(chunk2, chunk2_start),
        Err(PreContext(chunk0.len()))
    );
    c.provide_context(chunk0, 0);
    assert_eq!(c.next_boundary(chunk2, chunk2_start), Ok(Some(11)));
    assert_eq!(c.next_boundary(chunk2, chunk2_start), Ok(None));
}
