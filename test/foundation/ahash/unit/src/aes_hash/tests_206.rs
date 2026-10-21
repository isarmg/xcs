#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::Convert;
    use crate::operations::aesenc;
    use crate::RandomState;
    use std::hash::{BuildHasher, Hasher};
    #[test]
    fn test_sanity() {
        let mut hasher = RandomState::with_seeds(1, 2, 3, 4).build_hasher();
        hasher.write_u64(0);
        let h1 = hasher.finish();
        hasher.write(&[1, 0, 0, 0, 0, 0, 0, 0]);
        let h2 = hasher.finish();
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash() {
        let mut result: [u64; 2] = [0x6c62272e07bb0142, 0x62b821756295c58d];
        let value: [u64; 2] = [1 << 32, 0xFEDCBA9876543210];
        result = aesenc(value.convert(), result.convert()).convert();
        result = aesenc(result.convert(), result.convert()).convert();
        let mut result2: [u64; 2] = [0x6c62272e07bb0142, 0x62b821756295c58d];
        let value2: [u64; 2] = [1, 0xFEDCBA9876543210];
        result2 = aesenc(value2.convert(), result2.convert()).convert();
        result2 = aesenc(result2.convert(), result.convert()).convert();
        let result: [u8; 16] = result.convert();
        let result2: [u8; 16] = result2.convert();
        assert_ne!(hex::encode(result), hex::encode(result2));
    }
}
