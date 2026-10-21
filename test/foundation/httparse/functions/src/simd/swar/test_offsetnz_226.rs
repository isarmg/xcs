#[test]
fn test_offsetnz() {
    let seq = [0_u8; BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        let mut seq = seq;
        seq[i] = 1;
        let x = usize::from_ne_bytes(seq);
        assert_eq!(offsetnz(x), i);
    }
}
