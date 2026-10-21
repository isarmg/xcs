#[cfg(all(test, feature = "alloc"))]
mod test {
    use super::{
        decompress_slice_iter_to_slice, decompress_to_vec_zlib, decompress_to_vec_zlib_with_limit,
        DecompressError, TINFLStatus,
    };
    const ENCODED: [u8; 20] = [
        120, 156, 243, 72, 205, 201, 201, 215, 81, 168, 202, 201, 76, 82, 4, 0, 27, 101, 4, 19,
    ];

    #[test]
    fn decompress_vec() {
        let res = decompress_to_vec_zlib(&ENCODED[..]).unwrap();
        assert_eq!(res.as_slice(), &b"Hello, zlib!"[..]);
    }

    #[test]
    fn decompress_vec_with_high_limit() {
        let res = decompress_to_vec_zlib_with_limit(&ENCODED[..], 100_000).unwrap();
        assert_eq!(res.as_slice(), &b"Hello, zlib!"[..]);
    }

    #[test]
    fn fail_to_decompress_with_limit() {
        let res = decompress_to_vec_zlib_with_limit(&ENCODED[..], 8);
        match res {
            Err(DecompressError {
                status: TINFLStatus::HasMoreOutput,
                ..
            }) => (), // expected result
            _ => panic!("Decompression output size limit was not enforced"),
        }
    }

    #[test]
    fn test_decompress_slice_iter_to_slice() {
        // one slice
        let mut out = [0_u8; 12_usize];
        let r =
            decompress_slice_iter_to_slice(&mut out, Some(&ENCODED[..]).into_iter(), true, false);
        assert_eq!(r, Ok(12));
        assert_eq!(&out[..12], &b"Hello, zlib!"[..]);

        // some chunks at a time
        for chunk_size in 1..13 {
            // Note: because of https://github.com/Frommi/miniz_oxide/issues/110 our
            // out buffer needs to have +1 byte available when the chunk size cuts
            // the adler32 data off from the last actual data.
            let mut out = [0_u8; 12_usize + 1];
            let r =
                decompress_slice_iter_to_slice(&mut out, ENCODED.chunks(chunk_size), true, false);
            assert_eq!(r, Ok(12));
            assert_eq!(&out[..12], &b"Hello, zlib!"[..]);
        }

        // output buffer too small
        let mut out = [0_u8; 3_usize];
        let r = decompress_slice_iter_to_slice(&mut out, ENCODED.chunks(7), true, false);
        assert!(r.is_err());
    }
}
