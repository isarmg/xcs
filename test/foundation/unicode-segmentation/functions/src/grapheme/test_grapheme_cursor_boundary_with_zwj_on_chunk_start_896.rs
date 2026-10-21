#[test]
fn test_grapheme_cursor_boundary_with_zwj_on_chunk_start() {
    use GraphemeIncomplete::*;

    let chunk0 = "👩"; // 4 bytes
    let chunk1 = "\u{200d}🔬"; // 3 bytes + 4 bytes

    let full_len = chunk0.len() + chunk1.len();

    let mut cur = GraphemeCursor::new(0, full_len, true);
    assert_eq!(cur.next_boundary(chunk0, 0), Err(NextChunk));
    match cur.next_boundary(chunk1, chunk0.len()) {
        Ok(res) => assert_eq!(res, Some(11)),
        Err(PreContext(_)) => {
            cur.provide_context(chunk0, 0);
            assert_eq!(cur.next_boundary(chunk1, chunk0.len()), Ok(Some(11)));
        }
        _ => unreachable!(),
    }
}
