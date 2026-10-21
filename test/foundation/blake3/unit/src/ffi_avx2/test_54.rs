#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_hash_many() {
        if !crate::platform::avx2_detected() {
            return;
        }
        crate::test::test_hash_many_fn(hash_many, hash_many);
    }
}
