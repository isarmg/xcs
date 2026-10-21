#[cfg(all(test, target_arch = "aarch64", target_feature = "neon"))]
mod tests_aarch64_neon {
    use core::arch::aarch64::*;

    use super::*;

    #[target_feature(enable = "neon")]
    unsafe fn load(lanes: [u8; 16]) -> uint8x16_t {
        uint8x16_t::load_unaligned(&lanes as *const u8)
    }

    #[target_feature(enable = "neon")]
    unsafe fn unload(v: uint8x16_t) -> [u8; 16] {
        [
            vgetq_lane_u8(v, 0),
            vgetq_lane_u8(v, 1),
            vgetq_lane_u8(v, 2),
            vgetq_lane_u8(v, 3),
            vgetq_lane_u8(v, 4),
            vgetq_lane_u8(v, 5),
            vgetq_lane_u8(v, 6),
            vgetq_lane_u8(v, 7),
            vgetq_lane_u8(v, 8),
            vgetq_lane_u8(v, 9),
            vgetq_lane_u8(v, 10),
            vgetq_lane_u8(v, 11),
            vgetq_lane_u8(v, 12),
            vgetq_lane_u8(v, 13),
            vgetq_lane_u8(v, 14),
            vgetq_lane_u8(v, 15),
        ]
    }

    // Example functions. These don't test the Vector traits, but rather,
    // specific NEON instructions. They are basically little experiments I
    // wrote to figure out what an instruction does since their descriptions
    // are so dense. I decided to keep the experiments around as example tests
    // in case there' useful.

    #[test]
    fn example_vmaxvq_u8_non_zero() {
        #[target_feature(enable = "neon")]
        unsafe fn example() {
            let v = load([0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            assert_eq!(vmaxvq_u8(v), 1);
        }
        unsafe { example() }
    }

    #[test]
    fn example_vmaxvq_u8_zero() {
        #[target_feature(enable = "neon")]
        unsafe fn example() {
            let v = load([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            assert_eq!(vmaxvq_u8(v), 0);
        }
        unsafe { example() }
    }

    #[test]
    fn example_vpmaxq_u8_non_zero() {
        #[target_feature(enable = "neon")]
        unsafe fn example() {
            let v = load([0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            let r = vpmaxq_u8(v, v);
            assert_eq!(
                unload(r),
                [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0]
            );
        }
        unsafe { example() }
    }

    #[test]
    fn example_vpmaxq_u8_self() {
        #[target_feature(enable = "neon")]
        unsafe fn example() {
            let v =
                load([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
            let r = vpmaxq_u8(v, v);
            assert_eq!(
                unload(r),
                [2, 4, 6, 8, 10, 12, 14, 16, 2, 4, 6, 8, 10, 12, 14, 16]
            );
        }
        unsafe { example() }
    }

    #[test]
    fn example_vpmaxq_u8_other() {
        #[target_feature(enable = "neon")]
        unsafe fn example() {
            let v1 =
                load([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
            let v2 = load([
                17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
            ]);
            let r = vpmaxq_u8(v1, v2);
            assert_eq!(
                unload(r),
                [2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24, 26, 28, 30, 32]
            );
        }
        unsafe { example() }
    }

    // Now we test the actual methods on the Vector trait.

    #[test]
    fn vector_splat() {
        #[target_feature(enable = "neon")]
        unsafe fn test() {
            let v = uint8x16_t::splat(0xAF);
            assert_eq!(
                unload(v),
                [
                    0xAF, 0xAF, 0xAF, 0xAF, 0xAF, 0xAF, 0xAF, 0xAF, 0xAF,
                    0xAF, 0xAF, 0xAF, 0xAF, 0xAF, 0xAF, 0xAF
                ]
            );
        }
        unsafe { test() }
    }

    #[test]
    fn vector_is_zero() {
        #[target_feature(enable = "neon")]
        unsafe fn test() {
            let v = load([0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            assert!(!v.is_zero());
            let v = load([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            assert!(v.is_zero());
        }
        unsafe { test() }
    }

    #[test]
    fn vector_cmpeq() {
        #[target_feature(enable = "neon")]
        unsafe fn test() {
            let v1 =
                load([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 1]);
            let v2 =
                load([16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1]);
            assert_eq!(
                unload(v1.cmpeq(v2)),
                [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xFF]
            );
        }
        unsafe { test() }
    }

    #[test]
    fn vector_and() {
        #[target_feature(enable = "neon")]
        unsafe fn test() {
            let v1 =
                load([0, 0, 0, 0, 0, 0b1001, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            let v2 =
                load([0, 0, 0, 0, 0, 0b1010, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            assert_eq!(
                unload(v1.and(v2)),
                [0, 0, 0, 0, 0, 0b1000, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
            );
        }
        unsafe { test() }
    }

    #[test]
    fn vector_or() {
        #[target_feature(enable = "neon")]
        unsafe fn test() {
            let v1 =
                load([0, 0, 0, 0, 0, 0b1001, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            let v2 =
                load([0, 0, 0, 0, 0, 0b1010, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            assert_eq!(
                unload(v1.or(v2)),
                [0, 0, 0, 0, 0, 0b1011, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
            );
        }
        unsafe { test() }
    }

    #[test]
    fn vector_shift_8bit_lane_right() {
        #[target_feature(enable = "neon")]
        unsafe fn test() {
            let v = load([
                0, 0, 0, 0, 0b1011, 0b0101, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ]);
            assert_eq!(
                unload(v.shift_8bit_lane_right::<2>()),
                [0, 0, 0, 0, 0b0010, 0b0001, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
            );
        }
        unsafe { test() }
    }

    #[test]
    fn vector_shift_in_one_byte() {
        #[target_feature(enable = "neon")]
        unsafe fn test() {
            let v1 =
                load([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
            let v2 = load([
                17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
            ]);
            assert_eq!(
                unload(v1.shift_in_one_byte(v2)),
                [32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
            );
        }
        unsafe { test() }
    }

    #[test]
    fn vector_shift_in_two_bytes() {
        #[target_feature(enable = "neon")]
        unsafe fn test() {
            let v1 =
                load([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
            let v2 = load([
                17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
            ]);
            assert_eq!(
                unload(v1.shift_in_two_bytes(v2)),
                [31, 32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14],
            );
        }
        unsafe { test() }
    }

    #[test]
    fn vector_shift_in_three_bytes() {
        #[target_feature(enable = "neon")]
        unsafe fn test() {
            let v1 =
                load([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
            let v2 = load([
                17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
            ]);
            assert_eq!(
                unload(v1.shift_in_three_bytes(v2)),
                [30, 31, 32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
            );
        }
        unsafe { test() }
    }

    #[test]
    fn vector_shuffle_bytes() {
        #[target_feature(enable = "neon")]
        unsafe fn test() {
            let v1 =
                load([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
            let v2 =
                load([0, 0, 0, 0, 4, 4, 4, 4, 8, 8, 8, 8, 12, 12, 12, 12]);
            assert_eq!(
                unload(v1.shuffle_bytes(v2)),
                [1, 1, 1, 1, 5, 5, 5, 5, 9, 9, 9, 9, 13, 13, 13, 13],
            );
        }
        unsafe { test() }
    }

    #[test]
    fn vector_for_each_64bit_lane() {
        #[target_feature(enable = "neon")]
        unsafe fn test() {
            let v = load([
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A,
                0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10,
            ]);
            let mut lanes = [0u64; 2];
            v.for_each_64bit_lane(|i, lane| {
                lanes[i] = lane;
                None::<()>
            });
            assert_eq!(lanes, [0x0807060504030201, 0x100F0E0D0C0B0A09],);
        }
        unsafe { test() }
    }
}
