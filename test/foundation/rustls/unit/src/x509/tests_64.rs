#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;

    #[test]
    fn test_empty() {
        assert_eq!(vec![0x30, 0x00], wrap_in_sequence(&[]));
    }

    #[test]
    fn test_small() {
        assert_eq!(
            vec![0x30, 0x04, 0x00, 0x11, 0x22, 0x33],
            wrap_in_sequence(&[0x00, 0x11, 0x22, 0x33])
        );
    }

    #[test]
    fn test_medium() {
        let mut val = Vec::new();
        val.resize(255, 0x12);
        assert_eq!(
            vec![0x30, 0x81, 0xff, 0x12, 0x12, 0x12],
            wrap_in_sequence(&val)[..6]
        );
    }

    #[test]
    fn test_large() {
        let mut val = Vec::new();
        val.resize(4660, 0x12);
        wrap_in_sequence(&val);
        assert_eq!(
            vec![0x30, 0x82, 0x12, 0x34, 0x12, 0x12],
            wrap_in_sequence(&val)[..6]
        );
    }

    #[test]
    fn test_huge() {
        let mut val = Vec::new();
        val.resize(0xffff, 0x12);
        let result = wrap_in_sequence(&val);
        assert_eq!(vec![0x30, 0x82, 0xff, 0xff, 0x12, 0x12], result[..6]);
        assert_eq!(result.len(), 0xffff + 4);
    }

    #[test]
    fn test_gigantic() {
        let mut val = Vec::new();
        val.resize(0x100000, 0x12);
        let result = wrap_in_sequence(&val);
        assert_eq!(vec![0x30, 0x83, 0x10, 0x00, 0x00, 0x12, 0x12], result[..7]);
        assert_eq!(result.len(), 0x100000 + 5);
    }

    #[test]
    fn test_ludicrous() {
        let mut val = Vec::new();
        val.resize(0x1000000, 0x12);
        let result = wrap_in_sequence(&val);
        assert_eq!(
            vec![0x30, 0x84, 0x01, 0x00, 0x00, 0x00, 0x12, 0x12],
            result[..8]
        );
        assert_eq!(result.len(), 0x1000000 + 6);
    }

    #[test]
    fn test_wrap_in_bit_string() {
        // The BIT STRING encoding starts with a single octet on
        // the front saying how many bits to disregard from the
        // last octet. So this zero means "no bits" unused, which
        // is correct because our input is an string of octets.
        //
        // So if we encode &[0x55u8] with this function, we should get:
        //
        // 0x03    0x02    0x00                0x55
        // ^ tag   ^ len   ^ no unused bits    ^ value
        assert_eq!(wrap_in_bit_string(&[0x55u8]), vec![0x03, 0x02, 0x00, 0x55]);
    }
}
