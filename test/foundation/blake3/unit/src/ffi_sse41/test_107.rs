#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_compress() {
        if !crate::platform::sse41_detected() {
            return;
        }
        crate::test::test_compress_fn(compress_in_place, compress_xof);
    }

    #[test]
    fn test_hash_many() {
        if !crate::platform::sse41_detected() {
            return;
        }
        crate::test::test_hash_many_fn(hash_many, hash_many);
    }
}
