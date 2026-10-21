#[test]
fn test_hash_word_32() {
    assert_eq!(0, fxhash_32(b""));
    assert_eq!(0xF3051F19, fxhash_32(b"a"));
    assert_eq!(0x2F9DF119, fxhash_32(b"ab"));
    assert_eq!(0xCB1D9396, fxhash_32(b"abc"));
    assert_eq!(0x8628F119, fxhash_32(b"abcd"));
    assert_eq!(0xBEBDB56D, fxhash_32(b"abcde"));
    assert_eq!(0x1CE8476D, fxhash_32(b"abcdef"));
    assert_eq!(0xC0F176A4, fxhash_32(b"abcdefg"));
    assert_eq!(0x09AB476D, fxhash_32(b"abcdefgh"));
    assert_eq!(0xB72F5D88, fxhash_32(b"abcdefghi"));
}
