#[cfg(test)]
mod tests {
    use crate::io::{Read, Write};
    use alloc::string::{String, ToString};
    use alloc::vec::Vec;

    use super::{read, write, GzBuilder, GzHeaderParser};
    use crate::{Compression, GzHeader};
    use rand::{rng, Rng};

    #[test]
    fn roundtrip() {
        let mut e = write::GzEncoder::new(Vec::new(), Compression::default());
        e.write_all(b"foo bar baz").unwrap();
        let inner = e.finish().unwrap();
        let mut d = read::GzDecoder::new(&inner[..]);
        let mut s = String::new();
        d.read_to_string(&mut s).unwrap();
        assert_eq!(s, "foo bar baz");
    }

    #[test]
    fn roundtrip_zero() {
        let e = write::GzEncoder::new(Vec::new(), Compression::default());
        let inner = e.finish().unwrap();
        let mut d = read::GzDecoder::new(&inner[..]);
        let mut s = String::new();
        d.read_to_string(&mut s).unwrap();
        assert_eq!(s, "");
    }

    #[test]
    fn roundtrip_big() {
        let mut real = Vec::new();
        let mut w = write::GzEncoder::new(Vec::new(), Compression::default());
        let v = crate::random_bytes().take(1024).collect::<Vec<_>>();
        for _ in 0..200 {
            let to_write = &v[..rng().random_range(0..v.len())];
            real.extend(to_write.iter().copied());
            w.write_all(to_write).unwrap();
        }
        let result = w.finish().unwrap();
        let mut r = read::GzDecoder::new(&result[..]);
        let mut v = Vec::new();
        r.read_to_end(&mut v).unwrap();
        assert_eq!(v, real);
    }

    #[test]
    fn roundtrip_big2() {
        let v = crate::random_bytes().take(1024 * 1024).collect::<Vec<_>>();
        let mut r = read::GzDecoder::new(read::GzEncoder::new(&v[..], Compression::default()));
        let mut res = Vec::new();
        r.read_to_end(&mut res).unwrap();
        assert_eq!(res, v);
    }

    // A Rust implementation of CRC that closely matches the C code in RFC1952.
    // Only use this to create CRCs for tests.
    struct Rfc1952Crc {
        /* Table of CRCs of all 8-bit messages. */
        crc_table: [u32; 256],
    }

    impl Rfc1952Crc {
        fn new() -> Self {
            let mut crc = Rfc1952Crc {
                crc_table: [0; 256],
            };
            /* Make the table for a fast CRC. */
            for n in 0usize..256 {
                let mut c = n as u32;
                for _k in 0..8 {
                    if c & 1 != 0 {
                        c = 0xedb88320 ^ (c >> 1);
                    } else {
                        c >>= 1;
                    }
                }
                crc.crc_table[n] = c;
            }
            crc
        }

        /*
         Update a running crc with the bytes buf and return
         the updated crc. The crc should be initialized to zero. Pre- and
         post-conditioning (one's complement) is performed within this
         function so it shouldn't be done by the caller.
        */
        fn update_crc(&self, crc: u32, buf: &[u8]) -> u32 {
            let mut c = crc ^ 0xffffffff;

            for b in buf {
                c = self.crc_table[(c as u8 ^ *b) as usize] ^ (c >> 8);
            }
            c ^ 0xffffffff
        }

        /* Return the CRC of the bytes buf. */
        fn crc(&self, buf: &[u8]) -> u32 {
            self.update_crc(0, buf)
        }
    }

    #[test]
    fn roundtrip_header() {
        let mut header = GzBuilder::new()
            .mtime(1234)
            .operating_system(57)
            .filename("filename")
            .comment("comment")
            .into_header(Compression::fast());

        // Add a CRC to the header
        header[3] ^= super::FHCRC;
        let rfc1952_crc = Rfc1952Crc::new();
        let crc32 = rfc1952_crc.crc(&header);
        let crc16 = crc32 as u16;
        header.extend(&crc16.to_le_bytes());

        let mut parser = GzHeaderParser::new();
        parser.parse(&mut header.as_slice()).unwrap();
        let actual = parser.header().unwrap();
        assert_eq!(
            actual,
            &GzHeader {
                extra: None,
                filename: Some("filename".as_bytes().to_vec()),
                comment: Some("comment".as_bytes().to_vec()),
                operating_system: 57,
                mtime: 1234
            }
        )
    }

    #[test]
    fn gzip_encoder_matches_rfc1952() {
        /// Extract CRC32 and ISIZE from gzip footer
        fn extract_zip_footer(compressed: &[u8]) -> (u32, u32) {
            assert!(compressed.len() >= 8, "Gzip output too short");
            let footer_start = compressed.len() - 8;

            let crc = u32::from_le_bytes([
                compressed[footer_start],
                compressed[footer_start + 1],
                compressed[footer_start + 2],
                compressed[footer_start + 3],
            ]);

            let size = u32::from_le_bytes([
                compressed[footer_start + 4],
                compressed[footer_start + 5],
                compressed[footer_start + 6],
                compressed[footer_start + 7],
            ]);

            (crc, size)
        }

        #[track_caller]
        fn test_crc_for_write(data: &[u8], expected_crc: u32, description: &str) {
            // Compress data using write::GzEncoder
            let mut encoder = write::GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(data).unwrap();
            let compressed = encoder.finish().unwrap();

            let expected_size = data.len() as u32;
            let (actual_crc, actual_size) = extract_zip_footer(&compressed);

            assert_eq!(
                expected_crc, actual_crc,
                "CRC32 mismatch for write {}: expected {:#08x}, got {:#08x}",
                description, expected_crc, actual_crc
            );
            assert_eq!(
                expected_size, actual_size,
                "Size mismatch for write {}: expected {}, got {}",
                description, expected_size, actual_size
            );
        }

        #[track_caller]
        fn test_crc_for_read(data: &[u8], expected_crc: u32, description: &str) {
            // Compress data using read::GzEncoder
            let data_reader = crate::io::Cursor::new(data);
            let mut encoder = read::GzEncoder::new(data_reader, Compression::default());
            let mut compressed = Vec::new();
            encoder.read_to_end(&mut compressed).unwrap();

            let expected_size = data.len() as u32;
            let (actual_crc, actual_size) = extract_zip_footer(&compressed);

            assert_eq!(
                expected_crc, actual_crc,
                "CRC32 mismatch for read {}: expected {:#08x}, got {:#08x}",
                description, expected_crc, actual_crc
            );
            assert_eq!(
                expected_size, actual_size,
                "Size mismatch for read {}: expected {}, got {}",
                description, expected_size, actual_size
            );
        }

        #[track_caller]
        fn test_crc_for_data(data: &[u8], description: &str) {
            let rfc1952_crc = Rfc1952Crc::new();
            let expected_crc = rfc1952_crc.crc(data);

            test_crc_for_write(data, expected_crc, description);
            test_crc_for_read(data, expected_crc, description);
        }

        // Edge cases
        test_crc_for_data(&[], "empty data");
        test_crc_for_data(&[0x00], "single zero byte");
        test_crc_for_data(&[0xFF], "single 0xFF byte");

        // Simple text patterns
        test_crc_for_data(b"Hello World", "simple ASCII");
        test_crc_for_data(b"AAAAAAA", "repeated 'A'");
        test_crc_for_data(b"1234567890", "digits");

        // Binary patterns
        test_crc_for_data(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x05], "sequential bytes");
        test_crc_for_data(&[0xAA, 0x55, 0xAA, 0x55, 0xAA, 0x55], "alternating pattern");
        test_crc_for_data(&[0x00; 10], "all zeros");
        test_crc_for_data(&[0xFF; 10], "all ones");

        // Large data
        let large_data = vec![0x42; 10240];
        test_crc_for_data(&large_data, "10 kiB data");

        // Test multi-write scenario to ensure CRC accumulation works correctly
        {
            let data = b"This is a test of multi-write CRC accumulation";
            let rfc1952_crc = Rfc1952Crc::new();
            let expected_crc = rfc1952_crc.crc(data);

            let mut encoder = write::GzEncoder::new(Vec::new(), Compression::default());
            // Write in chunks
            encoder.write_all(&data[..10]).unwrap();
            encoder.write_all(&data[10..20]).unwrap();
            encoder.write_all(&data[20..]).unwrap();
            let compressed = encoder.finish().unwrap();

            let expected_size = data.len() as u32;
            let (actual_crc, actual_size) = extract_zip_footer(&compressed);

            assert_eq!(
                expected_crc, actual_crc,
                "Multi-write CRC mismatch: expected {:#08x}, got {:#08x}",
                expected_crc, actual_crc
            );
            assert_eq!(
                expected_size, actual_size,
                "Size mismatch for multi-write: expected {}, got {}",
                expected_size, actual_size
            );
        }
    }

    fn gzip_corrupted_crc() -> Vec<u8> {
        let test_data = b"The quick brown fox jumps over the lazy dog";

        let mut encoder = write::GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(test_data).unwrap();
        let mut compressed = encoder.finish().unwrap();

        // Corrupt the CRC32 in the footer
        let crc_offset = compressed.len() - 8;
        compressed[crc_offset] ^= 0xFF;

        compressed
    }

    #[test]
    fn read_decoder_detects_corrupted_crc() {
        let compressed = gzip_corrupted_crc();
        let mut decoder = read::GzDecoder::new(&compressed[..]);
        let mut output = Vec::new();
        let error = decoder.read_to_end(&mut output).unwrap_err();
        assert_eq!(error.kind(), crate::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn read_decoder_rejects_incomplete_deflate_stream() {
        let mut compressed = gzip_corrupted_crc();
        compressed.truncate(11);
        let error = read::GzDecoder::new(&compressed[..])
            .read_to_end(&mut Vec::new())
            .unwrap_err();
        assert_eq!(error.kind(), crate::io::ErrorKind::UnexpectedEof);
        assert_eq!(error.to_string(), "incomplete deflate stream");
    }

    #[test]
    fn write_decoder_detects_corrupted_crc() {
        let compressed = gzip_corrupted_crc();
        let mut decoder = write::GzDecoder::new(Vec::new());
        decoder.write_all(&compressed).unwrap();
        let error = decoder.finish().unwrap_err();
        assert_eq!(error.kind(), crate::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn fields() {
        let r = [0, 2, 4, 6];
        let e = GzBuilder::new()
            .filename("foo.rs")
            .comment("bar")
            .extra(vec![0, 1, 2, 3])
            .read(&r[..], Compression::default());
        let mut d = read::GzDecoder::new(e);
        assert_eq!(d.header().unwrap().filename(), Some(&b"foo.rs"[..]));
        assert_eq!(d.header().unwrap().comment(), Some(&b"bar"[..]));
        assert_eq!(d.header().unwrap().extra(), Some(&b"\x00\x01\x02\x03"[..]));
        let mut res = Vec::new();
        d.read_to_end(&mut res).unwrap();
        assert_eq!(res, vec![0, 2, 4, 6]);
    }

    #[test]
    #[should_panic(expected = "gzip extra field length cannot exceed u16::MAX")]
    fn extra_too_long() {
        GzBuilder::new().extra(vec![0; u16::MAX as usize + 1]);
    }

    #[test]
    fn keep_reading_after_end() {
        let mut e = write::GzEncoder::new(Vec::new(), Compression::default());
        e.write_all(b"foo bar baz").unwrap();
        let inner = e.finish().unwrap();
        let mut d = read::GzDecoder::new(&inner[..]);
        let mut s = String::new();
        d.read_to_string(&mut s).unwrap();
        assert_eq!(s, "foo bar baz");
        d.read_to_string(&mut s).unwrap();
        assert_eq!(s, "foo bar baz");
    }

    #[test]
    fn qc_reader() {
        ::quickcheck::quickcheck(test as fn(_) -> _);

        fn test(v: Vec<u8>) -> bool {
            let r = read::GzEncoder::new(&v[..], Compression::default());
            let mut r = read::GzDecoder::new(r);
            let mut v2 = Vec::new();
            r.read_to_end(&mut v2).unwrap();
            v == v2
        }
    }

    #[test]
    fn flush_after_write() {
        let mut f = write::GzEncoder::new(Vec::new(), Compression::default());
        write!(f, "Hello world").unwrap();
        f.flush().unwrap();
    }
}
