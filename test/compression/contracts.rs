use compression::checksum::{adler32_slice, Adler32};

#[test]
fn checksum_known_vectors_and_resume() {
    for (bytes, expected) in [
        (b"".as_slice(), 1),
        (b"Wikipedia", 0x11e60398),
        (b"123456789", 0x091e01de),
    ] {
        assert_eq!(adler32_slice(bytes), expected);
        for split in 0..=bytes.len() {
            let mut first = Adler32::new();
            first.write_slice(&bytes[..split]);
            let mut resumed = Adler32::from_checksum(first.checksum());
            resumed.write_slice(&bytes[split..]);
            assert_eq!(resumed.checksum(), expected);
        }
    }
}

#[cfg(feature = "simd")]
#[test]
fn accelerated_checksums_match_scalar_at_alignment_and_chunk_boundaries() {
    use compression::accelerated_checksum::{imp, Adler32 as Accelerated};
    // Exercise each supported implementation, not only the fastest dispatcher.
    let implementations: [Option<imp::Adler32Imp>; 4] = [
        Some(imp::scalar::update),
        imp::sse2::get_imp(),
        imp::ssse3::get_imp(),
        imp::avx2::get_imp(),
    ];
    for len in [0, 1, 15, 16, 31, 32, 63, 64, 5551, 5552, 5553, 22208, 65536] {
        let bytes: Vec<u8> = (0..len + 32)
            .map(|i| ((i * 31 + i / 7) % 256) as u8)
            .collect();
        for offset in 0..32 {
            let data = &bytes[offset..offset + len];
            let expected = adler32_slice(data);
            for update in implementations.into_iter().flatten() {
                let (a, b) = update(1, 0, data);
                assert_eq!((u32::from(b) << 16) | u32::from(a), expected);
            }
            let mut accelerated = Accelerated::new();
            for chunk in data.chunks(17) {
                accelerated.write(chunk);
            }
            assert_eq!(accelerated.finish(), expected);
            let mut resumed = Accelerated::from_checksum(expected);
            resumed.write(b"tail");
            let mut scalar = Adler32::from_checksum(expected);
            scalar.write_slice(b"tail");
            assert_eq!(resumed.finish(), scalar.checksum());
        }
    }
}

#[cfg(all(feature = "codec", feature = "alloc"))]
mod codec {
    use compression::codec::{deflate, inflate};

    #[test]
    fn raw_and_zlib_roundtrips_across_levels() {
        for data in [
            Vec::new(),
            b"hello".to_vec(),
            vec![0; 65536],
            (0..65536).map(|i| (i % 251) as u8).collect(),
        ] {
            for level in [0, 1, 6, 9, 10] {
                let raw = deflate::compress_to_vec(&data, level);
                assert_eq!(inflate::decompress_to_vec(&raw).unwrap(), data);
                let zlib = deflate::compress_to_vec_zlib(&data, level);
                assert_eq!(inflate::decompress_to_vec_zlib(&zlib).unwrap(), data);
            }
        }
    }

    #[test]
    fn independent_python_zlib_known_vectors() {
        let expected = b"hello";
        assert_eq!(
            inflate::decompress_to_vec(&[203, 72, 205, 201, 201, 7, 0]).unwrap(),
            expected
        );
        assert_eq!(
            inflate::decompress_to_vec_zlib(&[
                120, 156, 203, 72, 205, 201, 201, 7, 0, 6, 44, 2, 21
            ])
            .unwrap(),
            expected
        );
    }

    #[test]
    fn reject_invalid_truncated_and_checksum_corrupt_inputs() {
        for data in [&[][..], &[7], &[0, 255, 255, 0, 0], &[255, 255, 255, 255]] {
            assert!(inflate::decompress_to_vec(data).is_err(), "{data:?}");
        }
        let encoded = deflate::compress_to_vec_zlib(b"a payload requiring a complete trailer", 6);
        for end in 0..encoded.len() {
            assert!(
                inflate::decompress_to_vec_zlib(&encoded[..end]).is_err(),
                "prefix {end}"
            );
        }
        let mut corrupt = encoded.clone();
        *corrupt.last_mut().unwrap() ^= 1;
        assert!(inflate::decompress_to_vec_zlib(&corrupt).is_err());
        corrupt = encoded;
        corrupt[0] = 0;
        assert!(inflate::decompress_to_vec_zlib(&corrupt).is_err());
    }

    #[test]
    fn output_limit_is_enforced() {
        let encoded = deflate::compress_to_vec_zlib(&vec![42; 32768], 6);
        assert!(inflate::decompress_to_vec_zlib_with_limit(&encoded, 1024).is_err());
        assert_eq!(
            inflate::decompress_to_vec_zlib_with_limit(&encoded, 32768).unwrap(),
            vec![42; 32768]
        );
    }
}

#[cfg(feature = "streams")]
mod streams {
    use compression::streams::{self, Compression};
    use std::io::{Read, Write};

    #[test]
    fn all_formats_roundtrip_with_incremental_writes_and_flush() {
        let data: Vec<u8> = (0..70000).map(|i| (i % 251) as u8).collect();
        macro_rules! roundtrip {
            ($encoder:ty, $decoder:ty) => {{
                let mut encoder = <$encoder>::new(Vec::new(), Compression::default());
                for chunk in data.chunks(113) {
                    encoder.write_all(chunk).unwrap();
                    encoder.flush().unwrap();
                }
                let compressed = encoder.finish().unwrap();
                let mut decoder = <$decoder>::new(compressed.as_slice());
                let mut restored = Vec::new();
                let mut buffer = [0; 17];
                loop {
                    let n = decoder.read(&mut buffer).unwrap();
                    if n == 0 {
                        break;
                    }
                    restored.extend_from_slice(&buffer[..n]);
                }
                assert_eq!(restored, data);
            }};
        }
        roundtrip!(
            streams::write::DeflateEncoder<Vec<u8>>,
            streams::read::DeflateDecoder<&[u8]>
        );
        roundtrip!(
            streams::write::ZlibEncoder<Vec<u8>>,
            streams::read::ZlibDecoder<&[u8]>
        );
        roundtrip!(
            streams::write::GzEncoder<Vec<u8>>,
            streams::read::GzDecoder<&[u8]>
        );
    }

    fn gzip(data: &[u8]) -> Vec<u8> {
        let mut encoder = streams::write::GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).unwrap();
        encoder.finish().unwrap()
    }

    #[test]
    fn concatenated_gzip_and_trailing_bytes_contract() {
        let first = gzip(b"first");
        let second = gzip(b"second");
        let mut joined = first.clone();
        joined.extend_from_slice(&second);
        let mut result = Vec::new();
        streams::read::MultiGzDecoder::new(joined.as_slice())
            .read_to_end(&mut result)
            .unwrap();
        assert_eq!(result, b"firstsecond");
        joined.extend_from_slice(b"tail");
        let mut decoder = streams::bufread::GzDecoder::new(joined.as_slice());
        result.clear();
        decoder.read_to_end(&mut result).unwrap();
        assert_eq!(result, b"first");
        assert_eq!(decoder.into_inner(), &joined[first.len()..]);
    }

    #[test]
    fn gzip_rejects_each_truncation_and_corrupt_crc_or_length() {
        let compressed = gzip(b"payload");
        for end in 1..compressed.len() {
            let mut result = Vec::new();
            assert!(
                streams::read::GzDecoder::new(&compressed[..end])
                    .read_to_end(&mut result)
                    .is_err(),
                "prefix {end}"
            );
        }
        for position in [0, compressed.len() - 8, compressed.len() - 4] {
            let mut corrupt = compressed.clone();
            corrupt[position] ^= 1;
            assert!(streams::read::GzDecoder::new(corrupt.as_slice())
                .read_to_end(&mut Vec::new())
                .is_err());
        }
    }

    #[test]
    fn independent_python_gzip_vector() {
        let bytes = [
            31, 139, 8, 0, 0, 0, 0, 0, 2, 3, 203, 72, 205, 201, 201, 7, 0, 134, 166, 16, 54, 5, 0,
            0, 0,
        ];
        let mut restored = Vec::new();
        streams::read::GzDecoder::new(bytes.as_slice())
            .read_to_end(&mut restored)
            .unwrap();
        assert_eq!(restored, b"hello");
    }

    #[test]
    fn tiny_output_buffers_preserve_progress_counters() {
        use streams::{Compress, Decompress, FlushCompress, FlushDecompress, Status};
        let input = b"test data repeated test data repeated test data repeated";
        let mut compressor = Compress::new(Compression::default(), true);
        let mut output = Vec::new();
        loop {
            let before = compressor.total_out();
            let mut tiny = [0; 3];
            let status = compressor
                .compress(
                    &input[compressor.total_in() as usize..],
                    &mut tiny,
                    FlushCompress::Finish,
                )
                .unwrap();
            output.extend_from_slice(&tiny[..(compressor.total_out() - before) as usize]);
            if status == Status::StreamEnd {
                break;
            }
            assert!(compressor.total_out() > before);
        }
        assert_eq!(compressor.total_in(), input.len() as u64);
        assert_eq!(compressor.total_out(), output.len() as u64);
        let mut decoder = Decompress::new(true);
        let mut restored = Vec::new();
        loop {
            let before = decoder.total_out();
            let mut tiny = [0; 2];
            let status = decoder
                .decompress(
                    &output[decoder.total_in() as usize..],
                    &mut tiny,
                    FlushDecompress::None,
                )
                .unwrap();
            restored.extend_from_slice(&tiny[..(decoder.total_out() - before) as usize]);
            if status == Status::StreamEnd {
                break;
            }
            assert!(decoder.total_out() > before);
        }
        assert_eq!(restored, input);
        assert_eq!(decoder.total_in(), output.len() as u64);
    }
}
