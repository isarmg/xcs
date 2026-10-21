#[cfg(test)]
mod algorithm {
    use super::*;

    #[test]
    #[ignore]
    fn check() {
        fn is_hex_v1(c: u8) -> bool {
            matches!(c, b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F')
        }

        fn is_hex_v2(c: u8) -> bool {
            let x1 = c.wrapping_sub(0x30);
            let x2 = (c & 0xdf).wrapping_sub(0x41);
            x1 < 10 || x2 < 6
        }

        fn is_hex_v3(c: u8) -> bool {
            let x1 = c.wrapping_sub(0x30 + 0x80);
            let x2 = (c & 0xdf).wrapping_sub(0x41 + 0x80);
            ((x1 as i8) < -118) || ((x2 as i8) < -122)
        }

        for c in 0..=255_u8 {
            let (v1, v2, v3) = (is_hex_v1(c), is_hex_v2(c), is_hex_v3(c));
            assert_eq!(v1, v2);
            assert_eq!(v1, v3);
        }
    }

    #[test]
    #[ignore]
    fn hex_alsw() {
        HexAlsw::test_check();
        HexAlsw::test_decode();
    }
}
