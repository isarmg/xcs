#[cfg(test)]
mod test {
    use super::*;
    use zerovec::ZeroVec;

    #[test]
    fn test_serde_fail() {
        let uc = PotentialCodePoint([0xFF, 0xFF, 0xFF]);
        serde_json::to_string(&uc).expect_err("serialize invalid char bytes");
        bincode::serialize(&uc).expect_err("serialize invalid char bytes");
    }

    #[test]
    fn test_serde_json() {
        let c = '🙃';
        let uc = PotentialCodePoint::from_char(c);
        let json_ser = serde_json::to_string(&uc).unwrap();

        assert_eq!(json_ser, r#""🙃""#);

        let json_de: PotentialCodePoint = serde_json::from_str(&json_ser).unwrap();

        assert_eq!(uc, json_de);
    }

    #[test]
    fn test_serde_bincode() {
        let c = '🙃';
        let uc = PotentialCodePoint::from_char(c);
        let bytes_ser = bincode::serialize(&uc).unwrap();

        assert_eq!(bytes_ser, [0x43, 0xF6, 0x01]);

        let bytes_de: PotentialCodePoint = bincode::deserialize(&bytes_ser).unwrap();

        assert_eq!(uc, bytes_de);
    }

    #[test]
    fn test_representation() {
        let chars = ['w', 'ω', '文', '𑄃', '🙃'];

        // backed by [PotentialCodePoint]
        let uvchars: Vec<_> = chars
            .iter()
            .copied()
            .map(PotentialCodePoint::from_char)
            .collect();
        // backed by [RawBytesULE<3>]
        let zvec: ZeroVec<_> = uvchars.clone().into_iter().collect();

        let ule_bytes = zvec.as_bytes();
        let uvbytes;
        unsafe {
            let ptr = &uvchars[..] as *const _ as *const u8;
            uvbytes = core::slice::from_raw_parts(ptr, ule_bytes.len());
        }

        // PotentialCodePoint is defined as little-endian, so this must be true on all platforms
        // also asserts that to_unaligned/from_unaligned are no-ops
        assert_eq!(uvbytes, ule_bytes);

        assert_eq!(
            &[119, 0, 0, 201, 3, 0, 135, 101, 0, 3, 17, 1, 67, 246, 1],
            ule_bytes
        );
    }

    #[test]
    fn test_char_bake() {
        databake::test_bake!(
            PotentialCodePoint,
            const,
            crate::PotentialCodePoint::from_char('b'),
            potential_utf
        );
        // surrogate code point
        databake::test_bake!(
            PotentialCodePoint,
            const,
            crate::PotentialCodePoint::from_u24(55296u32),
            potential_utf
        );
    }
}
