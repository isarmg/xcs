#[cfg(test)]
mod tests {
    use crate::fallback_hash::*;

    #[test]
    fn test_hash() {
        let mut hasher = AHasher::new_with_keys(0, 0);
        let value: u64 = 1 << 32;
        hasher.update(value);
        let result = hasher.buffer;
        let mut hasher = AHasher::new_with_keys(0, 0);
        let value2: u64 = 1;
        hasher.update(value2);
        let result2 = hasher.buffer;
        let result: [u8; 8] = result.convert();
        let result2: [u8; 8] = result2.convert();
        assert_ne!(hex::encode(result), hex::encode(result2));
    }
}
