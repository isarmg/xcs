#[cfg(test)]
pub mod test {
    use super::*;

    // This is basically testing the portable implementation against itself,
    // but it also checks that compress_in_place and compress_xof are
    // consistent. And there are tests against the reference implementation and
    // against hardcoded test vectors elsewhere.
    #[test]
    fn test_compress() {
        crate::test::test_compress_fn(compress_in_place, compress_xof);
    }

    // Ditto.
    #[test]
    fn test_hash_many() {
        crate::test::test_hash_many_fn(hash_many, hash_many);
    }
}
