#[cfg(test)]
mod tests {
    use crate::io::Write;
    use alloc::vec::Vec;

    use crate::write;
    use crate::{Compression, Decompress, FlushDecompress};

    use crate::{Compress, FlushCompress};

    #[test]
    fn issue51() {
        let data = [
            0x1f, 0x8b, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0xb3, 0xc9, 0x28, 0xc9,
            0xcd, 0xb1, 0xe3, 0xe5, 0xb2, 0xc9, 0x48, 0x4d, 0x4c, 0xb1, 0xb3, 0x29, 0xc9, 0x2c,
            0xc9, 0x49, 0xb5, 0x33, 0x31, 0x30, 0x51, 0xf0, 0xcb, 0x2f, 0x51, 0x70, 0xcb, 0x2f,
            0xcd, 0x4b, 0xb1, 0xd1, 0x87, 0x08, 0xda, 0xe8, 0x83, 0x95, 0x00, 0x95, 0x26, 0xe5,
            0xa7, 0x54, 0x2a, 0x24, 0xa5, 0x27, 0xe7, 0xe7, 0xe4, 0x17, 0xd9, 0x2a, 0x95, 0x67,
            0x64, 0x96, 0xa4, 0x2a, 0x81, 0x8c, 0x48, 0x4e, 0xcd, 0x2b, 0x49, 0x2d, 0xb2, 0xb3,
            0xc9, 0x30, 0x44, 0x37, 0x01, 0x28, 0x62, 0xa3, 0x0f, 0x95, 0x06, 0xd9, 0x05, 0x54,
            0x04, 0xe5, 0xe5, 0xa5, 0x67, 0xe6, 0x55, 0xe8, 0x1b, 0xea, 0x99, 0xe9, 0x19, 0x21,
            0xab, 0xd0, 0x07, 0xd9, 0x01, 0x32, 0x53, 0x1f, 0xea, 0x3e, 0x00, 0x94, 0x85, 0xeb,
            0xe4, 0xa8, 0x00, 0x00, 0x00,
        ];

        let mut decoded = Vec::with_capacity(data.len() * 2);

        let mut d = Decompress::new(false);
        // decompressed whole deflate stream
        d.decompress_vec(&data[10..], &mut decoded, FlushDecompress::Finish)
            .unwrap();

        // decompress data that has nothing to do with the deflate stream (this
        // used to panic)
        drop(d.decompress_vec(&[0], &mut decoded, FlushDecompress::None));
    }

    #[test]
    fn reset() {
        let string = "hello world".as_bytes();
        let mut zlib = Vec::new();
        let mut deflate = Vec::new();

        let comp = Compression::default();
        write::ZlibEncoder::new(&mut zlib, comp)
            .write_all(string)
            .unwrap();
        write::DeflateEncoder::new(&mut deflate, comp)
            .write_all(string)
            .unwrap();

        let mut dst = [0; 1024];
        let mut decoder = Decompress::new(true);
        decoder
            .decompress(&zlib, &mut dst, FlushDecompress::Finish)
            .unwrap();
        assert_eq!(decoder.total_out(), string.len() as u64);
        assert!(dst.starts_with(string));

        decoder.reset(false);
        decoder
            .decompress(&deflate, &mut dst, FlushDecompress::Finish)
            .unwrap();
        assert_eq!(decoder.total_out(), string.len() as u64);
        assert!(dst.starts_with(string));
    }

    #[cfg(feature = "any_zlib")]
    #[test]
    fn test_gzip_flate() {
        let string = "hello, hello!".as_bytes();

        let mut encoded = Vec::with_capacity(1024);

        let mut encoder = Compress::new_gzip(Compression::default(), 9);

        encoder
            .compress_vec(string, &mut encoded, FlushCompress::Finish)
            .unwrap();

        assert_eq!(encoder.total_in(), string.len() as u64);
        assert_eq!(encoder.total_out(), encoded.len() as u64);

        let mut decoder = Decompress::new_gzip(9);

        let mut decoded = [0; 1024];
        decoder
            .decompress(&encoded, &mut decoded, FlushDecompress::Finish)
            .unwrap();

        assert_eq!(&decoded[..decoder.total_out() as usize], string);
    }

    #[cfg(feature = "any_zlib")]
    #[test]
    fn test_error_message() {
        let mut decoder = Decompress::new(false);
        let mut decoded = [0; 128];
        let garbage = b"xbvxzi";

        let err = decoder
            .decompress(garbage, &mut decoded, FlushDecompress::Finish)
            .unwrap_err();

        assert_eq!(err.message(), Some("invalid stored block lengths"));
    }

    fn compress_with_flush(flush: FlushCompress) -> Vec<u8> {
        let incompressible = (0..=255).collect::<Vec<u8>>();
        let mut output = vec![0; 1024];

        // Feed in the incompressible data followed by the indicated flush type.
        let mut w = Compress::new(Compression::default(), false);
        w.compress(&incompressible, &mut output, flush).unwrap();

        if flush != FlushCompress::None {
            // The first instance of incompressible input should have been written uncompressed.
            assert!(w.total_out() >= 261);
            assert_eq!(&output[0..5], &[0, 0, 1, 0xff, !1]);
            assert_eq!(&output[5..261], &incompressible);
        }

        // Feed in the same data again.
        let len = w.total_out() as usize;
        w.compress(&incompressible, &mut output[len..], FlushCompress::Finish)
            .unwrap();

        if flush != FlushCompress::Full {
            // This time, the data should have been compressed (because it is an exact duplicate of
            // the earlier block).
            assert!(w.total_out() < 300);
        }

        // Assert that all input has been processed.
        assert_eq!(w.total_in(), 256 * 2);

        output.resize(w.total_out() as usize, 0);
        output
    }

    #[test]
    fn test_partial_flush() {
        let output = compress_with_flush(FlushCompress::Partial);

        // Check for partial flush marker.
        assert_eq!(output[261], 0x2);
        assert_eq!(output[262] & 0x7, 0x4);
    }

    #[test]
    fn test_sync_flush() {
        let output = compress_with_flush(FlushCompress::Sync);

        // Check for sync flush marker.
        assert_eq!(&output[261..][..5], &[0, 0, 0, 0xff, 0xff]);
    }

    #[test]
    fn test_full_flush() {
        let output = compress_with_flush(FlushCompress::Full);
        assert_eq!(output.len(), 527);

        // Check for sync flush marker.
        assert_eq!(&output[261..][..5], &[0, 0, 0, 0xff, 0xff]);

        // Check that the second instance of incompressible input was also written uncompressed.
        assert_eq!(&output[266..][..5], &[1, 0, 1, 0xff, !1]);
    }
}
