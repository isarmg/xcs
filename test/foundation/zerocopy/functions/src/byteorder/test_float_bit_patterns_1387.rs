#[test]
    fn test_float_bit_patterns() {
        for bits in [0u32, 0x8000_0000, 0x7F80_0000, 0xFF80_0000, 0x7FC0_1234] {
            let be = F32::<BE>::new(f32::from_bits(bits));
            let le = F32::<LE>::new(f32::from_bits(bits));
            assert_eq!(be.to_bytes(), bits.to_be_bytes());
            assert_eq!(le.to_bytes(), bits.to_le_bytes());
            assert_eq!(be.get().to_bits(), bits);
            assert_eq!(le.get().to_bits(), bits);
        }
    }
