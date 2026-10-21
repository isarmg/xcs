#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_hash_many() {
        // This entire file is gated on feature="neon", so NEON support is
        // assumed here.
        crate::test::test_hash_many_fn(hash_many, hash_many);
    }
}
