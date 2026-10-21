#[test]
fn test_max_subtree_len() {
    assert_eq!(max_subtree_len(0), None);
    // (chunk index, max chunks)
    let cases = [
        (1, 1),
        (2, 2),
        (3, 1),
        (4, 4),
        (5, 1),
        (6, 2),
        (7, 1),
        (8, 8),
    ];
    for (chunk_index, max_chunks) in cases {
        let input_offset = chunk_index * CHUNK_LEN as u64;
        assert_eq!(
            max_subtree_len(input_offset),
            Some(max_chunks * CHUNK_LEN as u64),
        );
    }
}
