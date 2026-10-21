#[cfg(test)]
mod fallback_tests {
    use crate::fallback_hash::*;
    use crate::hash_quality_test::*;

    #[test]
    fn fallback_single_bit_flip() {
        test_single_bit_flip(|| AHasher::new_with_keys(0, 0))
    }

    #[test]
    fn fallback_single_key_bit_flip() {
        test_single_key_bit_flip(AHasher::new_with_keys)
    }

    #[test]
    fn fallback_all_bytes_matter() {
        test_all_bytes_matter(|| AHasher::new_with_keys(0, 0));
    }

    #[test]
    fn fallback_test_no_pair_collisions() {
        test_no_pair_collisions(|| AHasher::new_with_keys(0, 0));
    }

    #[test]
    fn fallback_test_no_full_collisions() {
        test_no_full_collisions(|| AHasher::new_with_keys(0, 0));
    }

    #[test]
    fn fallback_keys_change_output() {
        test_keys_change_output(AHasher::new_with_keys);
    }

    #[test]
    fn fallback_input_affect_every_byte() {
        test_input_affect_every_byte(AHasher::new_with_keys);
    }

    #[test]
    fn fallback_keys_affect_every_byte() {
        //For fallback second key is not used in every hash.
        #[cfg(all(folded_multiply))]
        test_keys_affect_every_byte(0, |a, b| AHasher::new_with_keys(a ^ b, a));
        test_keys_affect_every_byte("", |a, b| AHasher::new_with_keys(a ^ b, a));
        test_keys_affect_every_byte((0, 0), |a, b| AHasher::new_with_keys(a ^ b, a));
    }

    #[test]
    fn fallback_finish_is_consistant() {
        test_finish_is_consistent(AHasher::test_with_keys)
    }

    #[test]
    fn fallback_padding_doesnot_collide() {
        test_padding_doesnot_collide(|| AHasher::new_with_keys(0, 0));
        test_padding_doesnot_collide(|| AHasher::new_with_keys(0, 2));
        test_padding_doesnot_collide(|| AHasher::new_with_keys(2, 0));
        test_padding_doesnot_collide(|| AHasher::new_with_keys(2, 2));
    }

    #[test]
    fn fallback_length_extension() {
        test_length_extension(|a, b| AHasher::new_with_keys(a, b));
    }

    #[test]
    fn test_no_sparse_collisions() {
        test_sparse(|| AHasher::new_with_keys(0, 0));
        test_sparse(|| AHasher::new_with_keys(1, 2));
    }
}
