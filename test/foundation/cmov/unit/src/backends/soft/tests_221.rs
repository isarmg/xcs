#[cfg(test)]
mod tests {
    // Spot check up to a given limit
    const TEST_LIMIT: u32 = 65536;

    #[test]
    fn maskeq32() {
        assert_eq!(super::maskeq32(0, 0), u32::MAX);
        assert_eq!(super::maskeq32(1, 0), 0);
        assert_eq!(super::maskeq32(0, 1), 0);
        assert_eq!(super::maskeq32(1, 1), u32::MAX);
        assert_eq!(super::maskeq32(u32::MAX, 1), 0);
        assert_eq!(super::maskeq32(1, u32::MAX), 0);
        assert_eq!(super::maskeq32(u32::MAX, u32::MAX), u32::MAX);
    }

    #[test]
    fn maskeq64() {
        assert_eq!(super::maskeq64(0, 0), u64::MAX);
        assert_eq!(super::maskeq64(1, 0), 0);
        assert_eq!(super::maskeq64(0, 1), 0);
        assert_eq!(super::maskeq64(1, 1), u64::MAX);
        assert_eq!(super::maskeq64(u64::MAX, 1), 0);
        assert_eq!(super::maskeq64(1, u64::MAX), 0);
        assert_eq!(super::maskeq64(u64::MAX, u64::MAX), u64::MAX);
    }

    #[test]
    fn maskne32() {
        assert_eq!(super::maskne32(0, 0), 0);
        assert_eq!(super::maskne32(1, 0), u32::MAX);
        assert_eq!(super::maskne32(0, 1), u32::MAX);
        assert_eq!(super::maskne32(1, 1), 0);
        assert_eq!(super::maskne32(u32::MAX, 1), u32::MAX);
        assert_eq!(super::maskne32(1, u32::MAX), u32::MAX);
        assert_eq!(super::maskne32(u32::MAX, u32::MAX), 0);
    }

    #[test]
    fn maskne64() {
        assert_eq!(super::maskne64(0, 0), 0);
        assert_eq!(super::maskne64(1, 0), u64::MAX);
        assert_eq!(super::maskne64(0, 1), u64::MAX);
        assert_eq!(super::maskne64(1, 1), 0);
        assert_eq!(super::maskne64(u64::MAX, 1), u64::MAX);
        assert_eq!(super::maskne64(1, u64::MAX), u64::MAX);
        assert_eq!(super::maskne64(u64::MAX, u64::MAX), 0);
    }

    #[test]
    fn masknz32() {
        assert_eq!(super::masknz32(0), 0);
        for i in 1..=TEST_LIMIT {
            assert_eq!(super::masknz32(i), u32::MAX);
        }

        for i in (u32::MAX - TEST_LIMIT)..=u32::MAX {
            assert_eq!(super::masknz32(i), u32::MAX);
        }
    }

    #[test]
    fn masknz64() {
        assert_eq!(super::masknz64(0), 0);
        for i in 1..=(TEST_LIMIT as u64) {
            assert_eq!(super::masknz64(i), u64::MAX);
        }

        for i in (u64::MAX - TEST_LIMIT as u64)..=u64::MAX {
            assert_eq!(super::masknz64(i), u64::MAX);
        }
    }

    #[test]
    fn masksel() {
        assert_eq!(super::masksel(23u8, 42u8, 0u8), 23u8);
        assert_eq!(super::masksel(23u8, 42u8, u8::MAX), 42u8);

        assert_eq!(super::masksel(17u32, 101077u32, 0u32), 17u32);
        assert_eq!(super::masksel(17u32, 101077u32, u32::MAX), 101077u32);

        assert_eq!(super::masksel(129u64, 0xFFEEDDCCBBAA9988u64, 0u64), 129u64);
        assert_eq!(
            super::masksel(129u64, 0xFFEEDDCCBBAA9988u64, u64::MAX),
            0xFFEEDDCCBBAA9988u64
        );
    }
}
