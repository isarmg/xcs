#[cfg(test)]
mod aes_tests {
    use crate::aes_hash::*;
    use crate::hash_quality_test::*;
    use std::hash::{Hash, Hasher};

    //This encrypts to 0.
    const BAD_KEY2: u128 = 0x6363_6363_6363_6363_6363_6363_6363_6363;
    //This decrypts to 0.
    const BAD_KEY: u128 = 0x5252_5252_5252_5252_5252_5252_5252_5252;

    #[test]
    fn test_single_bit_in_byte() {
        let mut hasher1 = AHasher::test_with_keys(0, 0);
        8_u32.hash(&mut hasher1);
        let mut hasher2 = AHasher::test_with_keys(0, 0);
        0_u32.hash(&mut hasher2);
        assert_sufficiently_different(hasher1.finish(), hasher2.finish(), 1);
    }

    #[test]
    fn aes_single_bit_flip() {
        test_single_bit_flip(|| AHasher::test_with_keys(BAD_KEY, BAD_KEY));
        test_single_bit_flip(|| AHasher::test_with_keys(BAD_KEY2, BAD_KEY2));
    }

    #[test]
    fn aes_single_key_bit_flip() {
        test_single_key_bit_flip(AHasher::test_with_keys)
    }

    #[test]
    fn aes_all_bytes_matter() {
        test_all_bytes_matter(|| AHasher::test_with_keys(BAD_KEY, BAD_KEY));
        test_all_bytes_matter(|| AHasher::test_with_keys(BAD_KEY2, BAD_KEY2));
    }

    #[test]
    fn aes_test_no_pair_collisions() {
        test_no_pair_collisions(|| AHasher::test_with_keys(BAD_KEY, BAD_KEY));
        test_no_pair_collisions(|| AHasher::test_with_keys(BAD_KEY2, BAD_KEY2));
    }

    #[test]
    fn ase_test_no_full_collisions() {
        test_no_full_collisions(|| AHasher::test_with_keys(12345, 67890));
    }

    #[test]
    fn aes_keys_change_output() {
        test_keys_change_output(AHasher::test_with_keys);
    }

    #[test]
    fn aes_input_affect_every_byte() {
        test_input_affect_every_byte(AHasher::test_with_keys);
    }

    #[test]
    fn aes_keys_affect_every_byte() {

        test_keys_affect_every_byte(0, AHasher::test_with_keys);
        test_keys_affect_every_byte("", AHasher::test_with_keys);
        test_keys_affect_every_byte((0, 0), AHasher::test_with_keys);
    }

    #[test]
    fn aes_finish_is_consistant() {
        test_finish_is_consistent(AHasher::test_with_keys)
    }

    #[test]
    fn aes_padding_doesnot_collide() {
        test_padding_doesnot_collide(|| AHasher::test_with_keys(BAD_KEY, BAD_KEY));
        test_padding_doesnot_collide(|| AHasher::test_with_keys(BAD_KEY2, BAD_KEY2));
    }

    #[test]
    fn aes_length_extension() {
        test_length_extension(|a, b| AHasher::test_with_keys(a, b));
    }

    #[test]
    fn aes_no_sparse_collisions() {
        test_sparse(|| AHasher::test_with_keys(0, 0));
        test_sparse(|| AHasher::test_with_keys(1, 2));
    }
}
