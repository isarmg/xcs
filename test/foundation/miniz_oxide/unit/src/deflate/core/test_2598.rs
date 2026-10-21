#[cfg(test)]
mod test {
    use super::{
        compress_to_output, create_comp_flags_from_zip_params, read_u16_le, write_u16_le,
        CompressionStrategy, CompressorOxide, TDEFLFlush, TDEFLStatus, DEFAULT_FLAGS,
        MZ_DEFAULT_WINDOW_BITS,
    };
    use crate::inflate::decompress_to_vec;
    use alloc::vec;

    #[test]
    fn u16_to_slice() {
        let mut slice = [0, 0];
        write_u16_le(2000, &mut slice, 0);
        assert_eq!(slice, [208, 7]);
    }

    #[test]
    fn u16_from_slice() {
        let slice = [208, 7];
        assert_eq!(read_u16_le(&slice, 0), 2000);
    }

    #[test]
    fn compress_output() {
        assert_eq!(
            DEFAULT_FLAGS,
            create_comp_flags_from_zip_params(
                4,
                MZ_DEFAULT_WINDOW_BITS,
                CompressionStrategy::Default as i32
            )
        );

        let slice = [
            1, 2, 3, 4, 1, 2, 3, 1, 2, 3, 1, 2, 6, 1, 2, 3, 1, 2, 3, 2, 3, 1, 2, 3,
        ];
        let mut encoded = vec![];
        let flags = create_comp_flags_from_zip_params(6, 0, 0);
        let mut d = CompressorOxide::new(flags);
        let (status, in_consumed) =
            compress_to_output(&mut d, &slice, TDEFLFlush::Finish, |out: &[u8]| {
                encoded.extend_from_slice(out);
                true
            });

        assert_eq!(status, TDEFLStatus::Done);
        assert_eq!(in_consumed, slice.len());

        let decoded = decompress_to_vec(&encoded[..]).unwrap();
        assert_eq!(&decoded[..], &slice[..]);
    }

    #[test]
    /// Check fast compress mode
    fn compress_fast() {
        let slice = [
            1, 2, 3, 4, 1, 2, 3, 1, 2, 3, 1, 2, 6, 1, 2, 3, 1, 2, 3, 2, 3, 1, 2, 3,
        ];
        let mut encoded = vec![];
        let flags = create_comp_flags_from_zip_params(1, 0, 0);
        let mut d = CompressorOxide::new(flags);
        let (status, in_consumed) =
            compress_to_output(&mut d, &slice, TDEFLFlush::Finish, |out: &[u8]| {
                encoded.extend_from_slice(out);
                true
            });

        assert_eq!(status, TDEFLStatus::Done);
        assert_eq!(in_consumed, slice.len());

        // Needs to be altered if algorithm improves.
        assert_eq!(
            &encoded[..],
            [99, 100, 98, 102, 1, 98, 48, 98, 3, 147, 204, 76, 204, 140, 76, 204, 0]
        );

        let decoded = decompress_to_vec(&encoded[..]).unwrap();
        assert_eq!(&decoded[..], &slice[..]);
    }

    #[test]
    fn zlib_window_bits() {
        use super::TDEFL_RLE_MATCHES;
        use crate::deflate::CompressionLevel;
        use crate::inflate::stream::{inflate, InflateState};
        use crate::DataFormat;
        use alloc::boxed::Box;
        let slice = [
            1, 2, 3, 4, 1, 2, 3, 1, 2, 3, 1, 2, 6, 1, 2, 3, 1, 2, 3, 2, 3, 1, 2, 3, 35, 22, 22, 2,
            6, 2, 6,
        ];
        let mut encoded = vec![];

        // Create compressor with small window bits param
        let mut d = CompressorOxide::with_params(
            DataFormat::Zlib,
            CompressionLevel::DefaultCompression.into(),
            CompressionStrategy::Default,
            1,
        );

        assert!((d.params.flags & TDEFL_RLE_MATCHES) != 0);

        let (status, in_consumed) =
            compress_to_output(&mut d, &slice, TDEFLFlush::Finish, |out: &[u8]| {
                encoded.extend_from_slice(out);
                true
            });

        assert_eq!(status, TDEFLStatus::Done);
        assert_eq!(in_consumed, slice.len());

        let mut output = vec![0; slice.len()];

        let mut decompressor = Box::new(InflateState::new(DataFormat::Zlib));

        let mut out_slice = output.as_mut_slice();
        // Feed 1 byte at a time and no back buffer to test that RLE encoding has been used.
        for i in 0..encoded.len() {
            let result = inflate(
                &mut decompressor,
                &encoded[i..i + 1],
                out_slice,
                crate::MZFlush::None,
            );
            out_slice = &mut out_slice[result.bytes_written..];
        }
        let cmf = decompressor.decompressor().zlib_header().0;
        assert_eq!(cmf, 8);
        assert_eq!(output, slice)
    }
}
