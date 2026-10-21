#[cfg(all(test,any(feature = "brotli")))]
mod tests {
    use std::{println, vec};

    use super::*;



    #[test]
    #[cfg(feature = "brotli")]
    fn test_brotli() {
        test_compressor(BROTLI_COMPRESSOR, BROTLI_DECOMPRESSOR);
    }

    #[test]
    #[cfg(feature = "brotli")]
    fn test_brotli_rejects_large_window() {
        let large_window_empty = [0x11, 0xd0];
        brotli::BrotliDecompress(&mut &large_window_empty[..], &mut Vec::new()).unwrap();
        BROTLI_DECOMPRESSOR
            .decompress(&large_window_empty, &mut [])
            .unwrap_err();
    }

    fn test_compressor(comp: &dyn CertCompressor, decomp: &dyn CertDecompressor) {
        assert_eq!(comp.algorithm(), decomp.algorithm());
        for sz in [16, 64, 512, 2048, 8192, 16384] {
            test_trivial_pairwise(comp, decomp, sz);
        }
        test_decompress_wrong_len(comp, decomp);
        test_decompress_trailing_data(comp, decomp);
        test_decompress_garbage(decomp);
    }

    fn test_trivial_pairwise(
        comp: &dyn CertCompressor,
        decomp: &dyn CertDecompressor,
        plain_len: usize,
    ) {
        let original = vec![0u8; plain_len];

        for level in [CompressionLevel::Interactive, CompressionLevel::Amortized] {
            let compressed = comp
                .compress(original.clone(), level)
                .unwrap();
            println!(
                "{:?} compressed trivial {} -> {} using {:?} level",
                comp.algorithm(),
                original.len(),
                compressed.len(),
                level
            );
            let mut recovered = vec![0xffu8; plain_len];
            decomp
                .decompress(&compressed, &mut recovered)
                .unwrap();
            assert_eq!(original, recovered);
        }
    }

    fn test_decompress_wrong_len(comp: &dyn CertCompressor, decomp: &dyn CertDecompressor) {
        let original = vec![0u8; 2048];
        let compressed = comp
            .compress(original.clone(), CompressionLevel::Interactive)
            .unwrap();
        println!("{compressed:?}");

        // too big
        let mut recovered = vec![0xffu8; original.len() + 1];
        decomp
            .decompress(&compressed, &mut recovered)
            .unwrap_err();

        // too small
        let mut recovered = vec![0xffu8; original.len() - 1];
        decomp
            .decompress(&compressed, &mut recovered)
            .unwrap_err();
    }

    fn test_decompress_trailing_data(comp: &dyn CertCompressor, decomp: &dyn CertDecompressor) {
        let original = vec![0u8; 2048];
        let mut compressed = comp
            .compress(original.clone(), CompressionLevel::Interactive)
            .unwrap();
        compressed.extend_from_slice(b"trailing data");

        let mut recovered = vec![0xffu8; original.len()];
        decomp
            .decompress(&compressed, &mut recovered)
            .unwrap_err();
    }

    fn test_decompress_garbage(decomp: &dyn CertDecompressor) {
        let junk = [0u8; 1024];
        let mut recovered = vec![0u8; 512];
        decomp
            .decompress(&junk, &mut recovered)
            .unwrap_err();
    }


}
