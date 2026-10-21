#[cfg(test)]
mod tests {
    mod max_input {
        extern crate alloc;
        use super::super::super::digest;
        use crate::polyfill::u64_from_usize;
        use alloc::vec;

        macro_rules! max_input_tests {
            ( $algorithm_name:ident ) => {
                mod $algorithm_name {
                    use super::super::super::super::digest;

                    #[test]
                    fn max_input_test() {
                        super::max_input_test(&digest::$algorithm_name);
                    }

                    #[test]
                    #[should_panic]
                    fn too_long_input_test_block() {
                        super::too_long_input_test_block(&digest::$algorithm_name);
                    }

                    #[test]
                    #[should_panic]
                    fn too_long_input_test_byte() {
                        super::too_long_input_test_byte(&digest::$algorithm_name);
                    }
                }
            };
        }

        fn max_input_test(alg: &'static digest::Algorithm) {
            let mut context = nearly_full_context(alg);
            let next_input = vec![0u8; alg.block_len() - 1];
            context.update(&next_input);
            let _ = context.finish(); // no panic
        }

        fn too_long_input_test_block(alg: &'static digest::Algorithm) {
            let mut context = nearly_full_context(alg);
            let next_input = vec![0u8; alg.block_len()];
            context.update(&next_input);
            let _ = context.finish(); // should panic
        }

        fn too_long_input_test_byte(alg: &'static digest::Algorithm) {
            let mut context = nearly_full_context(alg);
            let next_input = vec![0u8; alg.block_len() - 1];
            context.update(&next_input);
            context.update(&[0]);
            let _ = context.finish(); // should panic
        }

        fn nearly_full_context(alg: &'static digest::Algorithm) -> digest::Context {
            // All implementations currently support up to 2^64-1 bits
            // of input; according to the spec, SHA-384 and SHA-512
            // support up to 2^128-1, but that's not implemented yet.
            let max_bytes = 1u64 << (64 - 3);
            let max_blocks = max_bytes / u64_from_usize(alg.block_len());
            let completed_bytes = (max_blocks - 1) * u64_from_usize(alg.block_len());
            digest::Context {
                block: digest::BlockContext {
                    state: alg.initial_state.clone(),
                    completed_bytes,
                    algorithm: alg,
                },
                pending: [0u8; digest::MAX_BLOCK_LEN],
                num_pending: 0,
            }
        }

        max_input_tests!(SHA1_FOR_LEGACY_USE_ONLY);
        max_input_tests!(SHA256);
        max_input_tests!(SHA384);
        max_input_tests!(SHA512);
    }
}
