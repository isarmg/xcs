#[cfg(test)]
mod test {
    use crate::streams::bufread::GzDecoder;
    use crate::streams::gz::write;
    use crate::streams::io::{Read, Write};
    use crate::streams::Compression;
    use alloc::vec::Vec;

    // GzDecoder consumes one gzip member and then returns 0 for subsequent reads, allowing any
    // additional data to be consumed by the caller.
    #[test]
    fn decode_extra_data() {
        let expected = "Hello World";

        let compressed = {
            let mut e = write::GzEncoder::new(Vec::new(), Compression::default());
            e.write_all(expected.as_ref()).unwrap();
            let mut b = e.finish().unwrap();
            b.push(b'x');
            b
        };

        let mut output = Vec::new();
        let mut decoder = GzDecoder::new(compressed.as_slice());
        let decoded_bytes = decoder.read_to_end(&mut output).unwrap();
        assert_eq!(decoded_bytes, output.len());
        let actual = core::str::from_utf8(&output).expect("String parsing error");
        assert_eq!(
            actual, expected,
            "after decompression we obtain the original input"
        );

        output.clear();
        assert_eq!(
            decoder.read(&mut output).unwrap(),
            0,
            "subsequent read of decoder returns 0, but inner reader can return additional data"
        );
        let mut reader = decoder.into_inner();
        assert_eq!(
            reader.read_to_end(&mut output).unwrap(),
            1,
            "extra data is accessible in underlying buf-read"
        );
        assert_eq!(output, b"x");
    }

    fn compress_data(data: &[u8]) -> Vec<u8> {
        use crate::streams::write::GzEncoder;
        use crate::streams::Compression;

        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        e.write_all(data).unwrap();
        e.finish().unwrap()
    }

    #[test]
    fn decode_with_reset() {
        let data1 = b"Hello World";
        let data2 = b"Goodbye World";

        let compressed1 = compress_data(data1);
        let compressed2 = compress_data(data2);

        let mut output = Vec::new();
        let mut decoder = GzDecoder::new(compressed1.as_slice());
        decoder.read_to_end(&mut output).unwrap();
        assert_eq!(output, data1);

        output.clear();
        decoder.reset(compressed2.as_slice());
        decoder.read_to_end(&mut output).unwrap();
        assert_eq!(output, data2);
    }

    #[test]
    fn decode_with_reset_after_corruption() {
        let valid_data = b"Hello World";
        let valid_compressed = compress_data(valid_data);

        // Create a corrupted payload (valid gzip header but corrupted body)
        let mut corrupted = valid_compressed.clone();
        assert!(corrupted.len() >= 14);
        corrupted[12] ^= 0xFF;
        corrupted[13] ^= 0xFF;

        // Try to decode corrupted data
        let mut decoder = GzDecoder::new(corrupted.as_slice());
        let mut output = Vec::new();
        let _ = decoder.read_to_end(&mut output).unwrap_err();

        // Reset with valid payload and decode
        decoder.reset(valid_compressed.as_slice());
        output.clear();
        decoder.read_to_end(&mut output).unwrap();
        assert_eq!(output, valid_data);
    }
}
