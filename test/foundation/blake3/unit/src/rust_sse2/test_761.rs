#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_transpose() {
        if !crate::platform::sse2_detected() {
            return;
        }

        #[target_feature(enable = "sse2")]
        unsafe fn transpose_wrapper(vecs: &mut [__m128i; DEGREE]) {
            unsafe { transpose_vecs(vecs) };
        }

        let mut matrix = [[0 as u32; DEGREE]; DEGREE];
        for i in 0..DEGREE {
            for j in 0..DEGREE {
                matrix[i][j] = (i * DEGREE + j) as u32;
            }
        }

        unsafe {
            let mut vecs: [__m128i; DEGREE] = core::mem::transmute(matrix);
            transpose_wrapper(&mut vecs);
            matrix = core::mem::transmute(vecs);
        }

        for i in 0..DEGREE {
            for j in 0..DEGREE {
                // Reversed indexes from above.
                assert_eq!(matrix[j][i], (i * DEGREE + j) as u32);
            }
        }
    }

    #[test]
    fn test_compress() {
        if !crate::platform::sse2_detected() {
            return;
        }
        crate::test::test_compress_fn(compress_in_place, compress_xof);
    }

    #[test]
    fn test_hash_many() {
        if !crate::platform::sse2_detected() {
            return;
        }
        crate::test::test_hash_many_fn(hash_many, hash_many);
    }
}
