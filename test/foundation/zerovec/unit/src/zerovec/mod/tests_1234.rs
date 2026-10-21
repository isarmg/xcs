#[cfg(test)]
mod tests {
    use super::*;
    use crate::samples::*;

    #[test]
    fn test_get() {
        {
            let zerovec = ZeroVec::from_slice_or_alloc(TEST_SLICE);
            assert_eq!(zerovec.get(0), Some(TEST_SLICE[0]));
            assert_eq!(zerovec.get(1), Some(TEST_SLICE[1]));
            assert_eq!(zerovec.get(2), Some(TEST_SLICE[2]));
        }
        {
            let zerovec = ZeroVec::<u32>::parse_bytes(TEST_BUFFER_LE).unwrap();
            assert_eq!(zerovec.get(0), Some(TEST_SLICE[0]));
            assert_eq!(zerovec.get(1), Some(TEST_SLICE[1]));
            assert_eq!(zerovec.get(2), Some(TEST_SLICE[2]));
        }
    }

    #[test]
    fn test_binary_search() {
        {
            let zerovec = ZeroVec::from_slice_or_alloc(TEST_SLICE);
            assert_eq!(Ok(3), zerovec.binary_search(&0x0e0d0c));
            assert_eq!(Err(3), zerovec.binary_search(&0x0c0d0c));
        }
        {
            let zerovec = ZeroVec::<u32>::parse_bytes(TEST_BUFFER_LE).unwrap();
            assert_eq!(Ok(3), zerovec.binary_search(&0x0e0d0c));
            assert_eq!(Err(3), zerovec.binary_search(&0x0c0d0c));
        }
    }

    #[test]
    fn test_odd_alignment() {
        assert_eq!(
            Some(0x020100),
            ZeroVec::<u32>::parse_bytes(TEST_BUFFER_LE).unwrap().get(0)
        );
        assert_eq!(
            Some(0x04000201),
            ZeroVec::<u32>::parse_bytes(&TEST_BUFFER_LE[1..77])
                .unwrap()
                .get(0)
        );
        assert_eq!(
            Some(0x05040002),
            ZeroVec::<u32>::parse_bytes(&TEST_BUFFER_LE[2..78])
                .unwrap()
                .get(0)
        );
        assert_eq!(
            Some(0x06050400),
            ZeroVec::<u32>::parse_bytes(&TEST_BUFFER_LE[3..79])
                .unwrap()
                .get(0)
        );
        assert_eq!(
            Some(0x060504),
            ZeroVec::<u32>::parse_bytes(&TEST_BUFFER_LE[4..])
                .unwrap()
                .get(0)
        );
        assert_eq!(
            Some(0x4e4d4c00),
            ZeroVec::<u32>::parse_bytes(&TEST_BUFFER_LE[75..79])
                .unwrap()
                .get(0)
        );
        assert_eq!(
            Some(0x4e4d4c00),
            ZeroVec::<u32>::parse_bytes(&TEST_BUFFER_LE[3..79])
                .unwrap()
                .get(18)
        );
        assert_eq!(
            Some(0x4e4d4c),
            ZeroVec::<u32>::parse_bytes(&TEST_BUFFER_LE[76..])
                .unwrap()
                .get(0)
        );
        assert_eq!(
            Some(0x4e4d4c),
            ZeroVec::<u32>::parse_bytes(TEST_BUFFER_LE).unwrap().get(19)
        );
        // TODO(#1144): Check for correct slice length in RawBytesULE
        // assert_eq!(
        //     None,
        //     ZeroVec::<u32>::parse_bytes(&TEST_BUFFER_LE[77..])
        //         .unwrap()
        //         .get(0)
        // );
        assert_eq!(
            None,
            ZeroVec::<u32>::parse_bytes(TEST_BUFFER_LE).unwrap().get(20)
        );
        assert_eq!(
            None,
            ZeroVec::<u32>::parse_bytes(&TEST_BUFFER_LE[3..79])
                .unwrap()
                .get(19)
        );
    }
}
