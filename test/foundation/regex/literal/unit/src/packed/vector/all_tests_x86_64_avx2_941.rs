#[cfg(all(test, target_arch = "x86_64", target_feature = "sse2"))]
mod tests_x86_64_avx2 {
    use core::arch::x86_64::*;

    use crate::literal::util::int::{I32, U32};

    use super::*;

    fn is_runnable() -> bool {
        std::is_x86_feature_detected!("avx2")
    }

    #[target_feature(enable = "avx2")]
    unsafe fn load(lanes: [u8; 32]) -> __m256i {
        __m256i::load_unaligned(&lanes as *const u8)
    }

    #[target_feature(enable = "avx2")]
    unsafe fn load_half(lanes: [u8; 16]) -> __m256i {
        __m256i::load_half_unaligned(&lanes as *const u8)
    }

    #[target_feature(enable = "avx2")]
    unsafe fn unload(v: __m256i) -> [u8; 32] {
        [
            _mm256_extract_epi8(v, 0).to_bits().low_u8(),
            _mm256_extract_epi8(v, 1).to_bits().low_u8(),
            _mm256_extract_epi8(v, 2).to_bits().low_u8(),
            _mm256_extract_epi8(v, 3).to_bits().low_u8(),
            _mm256_extract_epi8(v, 4).to_bits().low_u8(),
            _mm256_extract_epi8(v, 5).to_bits().low_u8(),
            _mm256_extract_epi8(v, 6).to_bits().low_u8(),
            _mm256_extract_epi8(v, 7).to_bits().low_u8(),
            _mm256_extract_epi8(v, 8).to_bits().low_u8(),
            _mm256_extract_epi8(v, 9).to_bits().low_u8(),
            _mm256_extract_epi8(v, 10).to_bits().low_u8(),
            _mm256_extract_epi8(v, 11).to_bits().low_u8(),
            _mm256_extract_epi8(v, 12).to_bits().low_u8(),
            _mm256_extract_epi8(v, 13).to_bits().low_u8(),
            _mm256_extract_epi8(v, 14).to_bits().low_u8(),
            _mm256_extract_epi8(v, 15).to_bits().low_u8(),
            _mm256_extract_epi8(v, 16).to_bits().low_u8(),
            _mm256_extract_epi8(v, 17).to_bits().low_u8(),
            _mm256_extract_epi8(v, 18).to_bits().low_u8(),
            _mm256_extract_epi8(v, 19).to_bits().low_u8(),
            _mm256_extract_epi8(v, 20).to_bits().low_u8(),
            _mm256_extract_epi8(v, 21).to_bits().low_u8(),
            _mm256_extract_epi8(v, 22).to_bits().low_u8(),
            _mm256_extract_epi8(v, 23).to_bits().low_u8(),
            _mm256_extract_epi8(v, 24).to_bits().low_u8(),
            _mm256_extract_epi8(v, 25).to_bits().low_u8(),
            _mm256_extract_epi8(v, 26).to_bits().low_u8(),
            _mm256_extract_epi8(v, 27).to_bits().low_u8(),
            _mm256_extract_epi8(v, 28).to_bits().low_u8(),
            _mm256_extract_epi8(v, 29).to_bits().low_u8(),
            _mm256_extract_epi8(v, 30).to_bits().low_u8(),
            _mm256_extract_epi8(v, 31).to_bits().low_u8(),
        ]
    }

    #[test]
    fn vector_splat() {
        #[target_feature(enable = "avx2")]
        unsafe fn test() {
            let v = __m256i::splat(0xAF);
            assert_eq!(
                unload(v),
                [
                    0xAF, 0xAF, 0xAF, 0xAF, 0xAF, 0xAF, 0xAF, 0xAF, 0xAF,
                    0xAF, 0xAF, 0xAF, 0xAF, 0xAF, 0xAF, 0xAF, 0xAF, 0xAF,
                    0xAF, 0xAF, 0xAF, 0xAF, 0xAF, 0xAF, 0xAF, 0xAF, 0xAF,
                    0xAF, 0xAF, 0xAF, 0xAF, 0xAF,
                ]
            );
        }
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }

    #[test]
    fn vector_is_zero() {
        #[target_feature(enable = "avx2")]
        unsafe fn test() {
            let v = load([
                0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ]);
            assert!(!v.is_zero());
            let v = load([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ]);
            assert!(v.is_zero());
        }
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }

    #[test]
    fn vector_cmpeq() {
        #[target_feature(enable = "avx2")]
        unsafe fn test() {
            let v1 = load([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
                19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 1,
            ]);
            let v2 = load([
                32, 31, 30, 29, 28, 27, 26, 25, 24, 23, 22, 21, 20, 19, 18,
                17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1,
            ]);
            assert_eq!(
                unload(v1.cmpeq(v2)),
                [
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xFF
                ]
            );
        }
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }

    #[test]
    fn vector_and() {
        #[target_feature(enable = "avx2")]
        unsafe fn test() {
            let v1 = load([
                0, 0, 0, 0, 0, 0b1001, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ]);
            let v2 = load([
                0, 0, 0, 0, 0, 0b1010, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ]);
            assert_eq!(
                unload(v1.and(v2)),
                [
                    0, 0, 0, 0, 0, 0b1000, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                ]
            );
        }
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }

    #[test]
    fn vector_or() {
        #[target_feature(enable = "avx2")]
        unsafe fn test() {
            let v1 = load([
                0, 0, 0, 0, 0, 0b1001, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ]);
            let v2 = load([
                0, 0, 0, 0, 0, 0b1010, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ]);
            assert_eq!(
                unload(v1.or(v2)),
                [
                    0, 0, 0, 0, 0, 0b1011, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                ]
            );
        }
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }

    #[test]
    fn vector_shift_8bit_lane_right() {
        #[target_feature(enable = "avx2")]
        unsafe fn test() {
            let v = load([
                0, 0, 0, 0, 0b1011, 0b0101, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ]);
            assert_eq!(
                unload(v.shift_8bit_lane_right::<2>()),
                [
                    0, 0, 0, 0, 0b0010, 0b0001, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                ]
            );
        }
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }

    #[test]
    fn vector_shift_in_one_byte() {
        #[target_feature(enable = "avx2")]
        unsafe fn test() {
            let v1 = load([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
                19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
            ]);
            let v2 = load([
                33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
                48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62,
                63, 64,
            ]);
            assert_eq!(
                unload(v1.shift_in_one_byte(v2)),
                [
                    64, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
                    17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30,
                    31,
                ],
            );
        }
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }

    #[test]
    fn vector_shift_in_two_bytes() {
        #[target_feature(enable = "avx2")]
        unsafe fn test() {
            let v1 = load([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
                19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
            ]);
            let v2 = load([
                33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
                48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62,
                63, 64,
            ]);
            assert_eq!(
                unload(v1.shift_in_two_bytes(v2)),
                [
                    63, 64, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
                    16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29,
                    30,
                ],
            );
        }
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }

    #[test]
    fn vector_shift_in_three_bytes() {
        #[target_feature(enable = "avx2")]
        unsafe fn test() {
            let v1 = load([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
                19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
            ]);
            let v2 = load([
                33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
                48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62,
                63, 64,
            ]);
            assert_eq!(
                unload(v1.shift_in_three_bytes(v2)),
                [
                    62, 63, 64, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
                    15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
                    29,
                ],
            );
        }
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }

    #[test]
    fn vector_shuffle_bytes() {
        #[target_feature(enable = "avx2")]
        unsafe fn test() {
            let v1 = load([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
                19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
            ]);
            let v2 = load([
                0, 0, 0, 0, 4, 4, 4, 4, 8, 8, 8, 8, 12, 12, 12, 12, 16, 16,
                16, 16, 20, 20, 20, 20, 24, 24, 24, 24, 28, 28, 28, 28,
            ]);
            assert_eq!(
                unload(v1.shuffle_bytes(v2)),
                [
                    1, 1, 1, 1, 5, 5, 5, 5, 9, 9, 9, 9, 13, 13, 13, 13, 17,
                    17, 17, 17, 21, 21, 21, 21, 25, 25, 25, 25, 29, 29, 29,
                    29
                ],
            );
        }
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }

    #[test]
    fn vector_for_each_64bit_lane() {
        #[target_feature(enable = "avx2")]
        unsafe fn test() {
            let v = load([
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A,
                0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11, 0x12, 0x13, 0x14,
                0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E,
                0x1F, 0x20,
            ]);
            let mut lanes = [0u64; 4];
            v.for_each_64bit_lane(|i, lane| {
                lanes[i] = lane;
                None::<()>
            });
            assert_eq!(
                lanes,
                [
                    0x0807060504030201,
                    0x100F0E0D0C0B0A09,
                    0x1817161514131211,
                    0x201F1E1D1C1B1A19
                ]
            );
        }
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }

    #[test]
    fn fat_vector_half_shift_in_one_byte() {
        #[target_feature(enable = "avx2")]
        unsafe fn test() {
            let v1 = load_half([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
            ]);
            let v2 = load_half([
                17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
            ]);
            assert_eq!(
                unload(v1.half_shift_in_one_byte(v2)),
                [
                    32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 32,
                    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15
                ],
            );
        }
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }

    #[test]
    fn fat_vector_half_shift_in_two_bytes() {
        #[target_feature(enable = "avx2")]
        unsafe fn test() {
            let v1 = load_half([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
            ]);
            let v2 = load_half([
                17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
            ]);
            assert_eq!(
                unload(v1.half_shift_in_two_bytes(v2)),
                [
                    31, 32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 31,
                    32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14,
                ],
            );
        }
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }

    #[test]
    fn fat_vector_half_shift_in_three_bytes() {
        #[target_feature(enable = "avx2")]
        unsafe fn test() {
            let v1 = load_half([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
            ]);
            let v2 = load_half([
                17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
            ]);
            assert_eq!(
                unload(v1.half_shift_in_three_bytes(v2)),
                [
                    30, 31, 32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 30,
                    31, 32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13,
                ],
            );
        }
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }

    #[test]
    fn fat_vector_swap_halves() {
        #[target_feature(enable = "avx2")]
        unsafe fn test() {
            let v = load([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
                19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
            ]);
            assert_eq!(
                unload(v.swap_halves()),
                [
                    17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30,
                    31, 32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
                    16,
                ],
            );
        }
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }

    #[test]
    fn fat_vector_interleave_low_8bit_lanes() {
        #[target_feature(enable = "avx2")]
        unsafe fn test() {
            let v1 = load([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
                19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
            ]);
            let v2 = load([
                33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
                48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62,
                63, 64,
            ]);
            assert_eq!(
                unload(v1.interleave_low_8bit_lanes(v2)),
                [
                    1, 33, 2, 34, 3, 35, 4, 36, 5, 37, 6, 38, 7, 39, 8, 40,
                    17, 49, 18, 50, 19, 51, 20, 52, 21, 53, 22, 54, 23, 55,
                    24, 56,
                ],
            );
        }
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }

    #[test]
    fn fat_vector_interleave_high_8bit_lanes() {
        #[target_feature(enable = "avx2")]
        unsafe fn test() {
            let v1 = load([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
                19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
            ]);
            let v2 = load([
                33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
                48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62,
                63, 64,
            ]);
            assert_eq!(
                unload(v1.interleave_high_8bit_lanes(v2)),
                [
                    9, 41, 10, 42, 11, 43, 12, 44, 13, 45, 14, 46, 15, 47, 16,
                    48, 25, 57, 26, 58, 27, 59, 28, 60, 29, 61, 30, 62, 31,
                    63, 32, 64,
                ],
            );
        }
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }

    #[test]
    fn fat_vector_for_each_low_64bit_lane() {
        #[target_feature(enable = "avx2")]
        unsafe fn test() {
            let v1 = load([
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A,
                0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11, 0x12, 0x13, 0x14,
                0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E,
                0x1F, 0x20,
            ]);
            let v2 = load([
                0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A,
                0x2B, 0x2C, 0x2D, 0x2E, 0x2F, 0x30, 0x31, 0x32, 0x33, 0x34,
                0x35, 0x36, 0x37, 0x38, 0x39, 0x3A, 0x3B, 0x3C, 0x3D, 0x3E,
                0x3F, 0x40,
            ]);
            let mut lanes = [0u64; 4];
            v1.for_each_low_64bit_lane(v2, |i, lane| {
                lanes[i] = lane;
                None::<()>
            });
            assert_eq!(
                lanes,
                [
                    0x0807060504030201,
                    0x100F0E0D0C0B0A09,
                    0x2827262524232221,
                    0x302F2E2D2C2B2A29
                ]
            );
        }
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }
}
