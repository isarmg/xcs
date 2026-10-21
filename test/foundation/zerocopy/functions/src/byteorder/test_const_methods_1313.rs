#[test]
    fn test_const_methods() {
        use big_endian::*;

        #[rustversion::since(1.61.0)]
        const _U: U16 = U16::new(0);
        #[rustversion::since(1.61.0)]
        const _NATIVE: u16 = _U.get();
        const _FROM_BYTES: U16 = U16::from_bytes([0, 1]);
        const _BYTES: [u8; 2] = _FROM_BYTES.to_bytes();
    }
