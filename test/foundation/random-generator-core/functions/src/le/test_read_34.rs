#[test]
fn test_read() {
    let bytes = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

    let mut buf = [0u32; 4];
    read_u32_into(&bytes, &mut buf);
    assert_eq!(buf[0], 0x04030201);
    assert_eq!(buf[3], 0x100F0E0D);

    let mut buf = [0u32; 3];
    read_u32_into(&bytes[1..13], &mut buf); // unaligned
    assert_eq!(buf[0], 0x05040302);
    assert_eq!(buf[2], 0x0D0C0B0A);

    let mut buf = [0u64; 2];
    read_u64_into(&bytes, &mut buf);
    assert_eq!(buf[0], 0x0807060504030201);
    assert_eq!(buf[1], 0x100F0E0D0C0B0A09);

    let mut buf = [0u64; 1];
    read_u64_into(&bytes[7..15], &mut buf); // unaligned
    assert_eq!(buf[0], 0x0F0E0D0C0B0A0908);
}
