#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_transpose() {
        #[target_feature(enable = "simd128")]
        fn transpose_wrapper(vecs: &mut [v128; DEGREE]) {
            transpose_vecs(vecs);
        }

        let mut matrix = [[0 as u32; DEGREE]; DEGREE];
        for i in 0..DEGREE {
            for j in 0..DEGREE {
                matrix[i][j] = (i * DEGREE + j) as u32;
            }
        }

        unsafe {
            let mut vecs: [v128; DEGREE] = core::mem::transmute(matrix);
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
        crate::test::test_compress_fn(compress_in_place, compress_xof);
    }

    #[test]
    fn test_hash_many() {
        crate::test::test_hash_many_fn(hash_many, hash_many);
    }
}
