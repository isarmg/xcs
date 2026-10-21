#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bssl, rand};

    fn leak_in_test(a: BoolMask) -> bool {
        a.leak()
    }

    #[test]
    fn test_constant_time() -> Result<(), error::Unspecified> {
        prefixed_extern! {
            fn bssl_constant_time_test_main() -> bssl::Result;
        }
        Result::from(unsafe { bssl_constant_time_test_main() })
    }

    #[test]
    fn constant_time_conditional_memcpy() -> Result<(), error::Unspecified> {
        let rng = rand::SystemRandom::new();
        for _ in 0..100 {
            let mut out = rand::generate::<[u8; 256]>(&rng)?.expose();
            let input = rand::generate::<[u8; 256]>(&rng)?.expose();

            // Mask to 16 bits to make zero more likely than it would otherwise be.
            let b = (rand::generate::<[u8; 1]>(&rng)?.expose()[0] & 0x0f) == 0;

            let ref_in = input;
            let ref_out = if b { input } else { out };

            prefixed_extern! {
                fn bssl_constant_time_test_conditional_memcpy(dst: &mut [u8; 256], src: &[u8; 256], b: BoolMask);
            }
            unsafe {
                bssl_constant_time_test_conditional_memcpy(
                    &mut out,
                    &input,
                    if b { BoolMask::TRUE } else { BoolMask::FALSE },
                )
            }
            assert_eq!(ref_in, input);
            assert_eq!(ref_out, out);
        }

        Ok(())
    }

    #[test]
    fn constant_time_conditional_memxor() -> Result<(), error::Unspecified> {
        let rng = rand::SystemRandom::new();
        for _ in 0..256 {
            let mut out = rand::generate::<[u8; 256]>(&rng)?.expose();
            let input = rand::generate::<[u8; 256]>(&rng)?.expose();

            // Mask to 16 bits to make zero more likely than it would otherwise be.
            let b = (rand::generate::<[u8; 1]>(&rng)?.expose()[0] & 0x0f) != 0;

            let ref_in = input;
            let mut ref_out = out;
            if b {
                xor_assign_at_start(&mut ref_out, &ref_in)
            };

            prefixed_extern! {
                fn bssl_constant_time_test_conditional_memxor(dst: &mut [u8; 256], src: &[u8; 256], b: BoolMask);
            }
            unsafe {
                bssl_constant_time_test_conditional_memxor(
                    &mut out,
                    &input,
                    if b { BoolMask::TRUE } else { BoolMask::FALSE },
                );
            }

            assert_eq!(ref_in, input);
            assert_eq!(ref_out, out);
        }

        Ok(())
    }

    #[test]
    fn test_bool_mask_bitwise_and_is_logical_and() {
        assert!(leak_in_test(BoolMask::TRUE & BoolMask::TRUE));
        assert!(!leak_in_test(BoolMask::TRUE & BoolMask::FALSE));
        assert!(!leak_in_test(BoolMask::FALSE & BoolMask::TRUE));
        assert!(!leak_in_test(BoolMask::FALSE & BoolMask::FALSE));
    }
}
