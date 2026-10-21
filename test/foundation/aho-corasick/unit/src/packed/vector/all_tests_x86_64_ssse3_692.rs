#[cfg(all(test, target_arch = "x86_64", target_feature = "sse2"))]
mod tests_x86_64_ssse3 {
    use core::arch::x86_64::*;

    use crate::util::int::{I32, U32};

    use super::*;

    fn is_runnable() -> bool {
        std::is_x86_feature_detected!("ssse3")
    }

    #[target_feature(enable = "ssse3")]
    unsafe fn load(lanes: [u8; 16]) -> __m128i {
        __m128i::load_unaligned(&lanes as *const u8)
    }

    #[target_feature(enable = "ssse3")]
    unsafe fn unload(v: __m128i) -> [u8; 16] {
        [
            _mm_extract_epi8(v, 0).to_bits().low_u8(),
            _mm_extract_epi8(v, 1).to_bits().low_u8(),
            _mm_extract_epi8(v, 2).to_bits().low_u8(),
            _mm_extract_epi8(v, 3).to_bits().low_u8(),
            _mm_extract_epi8(v, 4).to_bits().low_u8(),
            _mm_extract_epi8(v, 5).to_bits().low_u8(),
            _mm_extract_epi8(v, 6).to_bits().low_u8(),
            _mm_extract_epi8(v, 7).to_bits().low_u8(),
            _mm_extract_epi8(v, 8).to_bits().low_u8(),
            _mm_extract_epi8(v, 9).to_bits().low_u8(),
            _mm_extract_epi8(v, 10).to_bits().low_u8(),
            _mm_extract_epi8(v, 11).to_bits().low_u8(),
            _mm_extract_epi8(v, 12).to_bits().low_u8(),
            _mm_extract_epi8(v, 13).to_bits().low_u8(),
            _mm_extract_epi8(v, 14).to_bits().low_u8(),
            _mm_extract_epi8(v, 15).to_bits().low_u8(),
        ]
    }

    #[test]
    fn vector_splat() {
        #[target_feature(enable = "ssse3")]
        unsafe fn test() {
            let v = __m128i::splat(0xAF);
            assert_eq!(
                unload(v),
                [
                    0xAF, 0xAF, 0xAF, 0xAF, 0xAF, 0xAF, 0xAF, 0xAF, 0xAF,
                    0xAF, 0xAF, 0xAF, 0xAF, 0xAF, 0xAF, 0xAF
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
        #[target_feature(enable = "ssse3")]
        unsafe fn test() {
            let v = load([0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            assert!(!v.is_zero());
            let v = load([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
            assert!(v.is_zero());
        }
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }

    #[test]
    fn vector_cmpeq() {
        #[target_feature(enable = "ssse3")]
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
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }

    #[test]
    fn vector_and() {
        #[target_feature(enable = "ssse3")]
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
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }

    #[test]
    fn vector_or() {
        #[target_feature(enable = "ssse3")]
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
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }

    #[test]
    fn vector_shift_8bit_lane_right() {
        #[target_feature(enable = "ssse3")]
        unsafe fn test() {
            let v = load([
                0, 0, 0, 0, 0b1011, 0b0101, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            ]);
            assert_eq!(
                unload(v.shift_8bit_lane_right::<2>()),
                [0, 0, 0, 0, 0b0010, 0b0001, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
            );
        }
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }

    #[test]
    fn vector_shift_in_one_byte() {
        #[target_feature(enable = "ssse3")]
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
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }

    #[test]
    fn vector_shift_in_two_bytes() {
        #[target_feature(enable = "ssse3")]
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
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }

    #[test]
    fn vector_shift_in_three_bytes() {
        #[target_feature(enable = "ssse3")]
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
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }

    #[test]
    fn vector_shuffle_bytes() {
        #[target_feature(enable = "ssse3")]
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
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }

    #[test]
    fn vector_for_each_64bit_lane() {
        #[target_feature(enable = "ssse3")]
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
        if !is_runnable() {
            return;
        }
        unsafe { test() }
    }
}
