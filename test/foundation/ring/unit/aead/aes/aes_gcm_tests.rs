#[cfg(test)]
mod aes_gcm_tests {
    use super::{super::aes_gcm::MAX_IN_OUT_LEN, *};
    use core::num::NonZeroU32;

    #[test]
    fn test_aes_gcm_counter_blocks_max() {
        test_aes_gcm_counter_blocks(MAX_IN_OUT_LEN, &[0, 0, 0, 0]);
    }

    #[test]
    fn test_aes_gcm_counter_blocks_max_minus_one() {
        test_aes_gcm_counter_blocks(MAX_IN_OUT_LEN - BLOCK_LEN, &[0xff, 0xff, 0xff, 0xff]);
    }

    fn test_aes_gcm_counter_blocks(in_out_len: usize, expected_final_counter: &[u8; 4]) {
        fn ctr32(ctr: &Counter) -> &[u8; 4] {
            (&ctr.0[12..]).try_into().unwrap()
        }

        let rounded_down = in_out_len / BLOCK_LEN;
        let blocks = rounded_down + (if in_out_len % BLOCK_LEN == 0 { 0 } else { 1 });
        let blocks = u32::try_from(blocks)
            .ok()
            .and_then(NonZeroU32::new)
            .unwrap();

        let nonce = Nonce::assume_unique_for_key([1; 12]);
        let mut ctr = Counter::one(nonce);
        assert_eq!(ctr32(&ctr), &[0, 0, 0, 1]);
        let _tag_iv = ctr.increment();
        assert_eq!(ctr32(&ctr), &[0, 0, 0, 2]);
        ctr.increment_by_less_safe(blocks);

        // `MAX_IN_OUT_LEN` is less on 32-bit targets, so we don't even get
        // close to wrapping, but run the tests on them anyway.
        #[cfg(target_pointer_width = "64")]
        assert_eq!(ctr32(&ctr), expected_final_counter);

        #[cfg(target_pointer_width = "32")]
        let _ = expected_final_counter;
    }
}
