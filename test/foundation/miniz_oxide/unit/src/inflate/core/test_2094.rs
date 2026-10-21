#[cfg(test)]
mod test {
    use super::*;

    //TODO: Fix these.

    fn tinfl_decompress_oxide<'i>(
        r: &mut DecompressorOxide,
        input_buffer: &'i [u8],
        output_buffer: &mut [u8],
        flags: u32,
    ) -> (TINFLStatus, &'i [u8], usize) {
        let (status, in_pos, out_pos) = decompress(r, input_buffer, output_buffer, 0, flags);
        (status, &input_buffer[in_pos..], out_pos)
    }

    #[test]
    fn decompress_zlib() {
        let encoded = [
            120, 156, 243, 72, 205, 201, 201, 215, 81, 168, 202, 201, 76, 82, 4, 0, 27, 101, 4, 19,
        ];
        let flags = TINFL_FLAG_COMPUTE_ADLER32 | TINFL_FLAG_PARSE_ZLIB_HEADER;

        let mut b = DecompressorOxide::new();
        const LEN: usize = 32;
        let mut b_buf = [0; LEN];

        // This should fail with the out buffer being to small.
        let b_status = tinfl_decompress_oxide(&mut b, &encoded[..], &mut b_buf, flags);

        assert!(b_status.0 == TINFLStatus::Failed);

        let flags = flags | TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF;

        b = DecompressorOxide::new();

        // With TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF set this should no longer fail.
        let b_status = tinfl_decompress_oxide(&mut b, &encoded[..], &mut b_buf, flags);

        assert_eq!(b_buf[..b_status.2], b"Hello, zlib!"[..]);
        assert!(b_status.0 == TINFLStatus::Done);
    }

    #[cfg(feature = "with-alloc")]
    #[test]
    fn raw_block() {
        const LEN: usize = 64;

        let text = b"Hello, zlib!";
        let encoded = {
            let len = text.len();
            let notlen = !len;
            let mut encoded = vec![
                1,
                len as u8,
                (len >> 8) as u8,
                notlen as u8,
                (notlen >> 8) as u8,
            ];
            encoded.extend_from_slice(&text[..]);
            encoded
        };

        //let flags = TINFL_FLAG_COMPUTE_ADLER32 | TINFL_FLAG_PARSE_ZLIB_HEADER |
        let flags = TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF;

        let mut b = DecompressorOxide::new();

        let mut b_buf = [0; LEN];

        let b_status = tinfl_decompress_oxide(&mut b, &encoded[..], &mut b_buf, flags);
        assert_eq!(b_buf[..b_status.2], text[..]);
        assert_eq!(b_status.0, TINFLStatus::Done);
    }

    fn masked_lookup(table: &HuffmanTable, bit_buf: BitBuffer) -> (i32, u32) {
        let ret = table.lookup(bit_buf);
        (ret.0 & 511, ret.1)
    }

    #[test]
    fn fixed_table_lookup() {
        let mut d = DecompressorOxide::new();
        d.block_type = 1;
        start_static_table(&mut d);
        let mut l = LocalVars {
            bit_buf: d.bit_buf,
            num_bits: d.num_bits,
            dist: d.dist,
            counter: d.counter,
            num_extra: d.num_extra,
        };
        init_tree(&mut d, &mut l).unwrap();
        let llt = &d.tables[LITLEN_TABLE];
        let dt = &d.tables[DIST_TABLE];
        assert_eq!(masked_lookup(llt, 0b00001100), (0, 8));
        assert_eq!(masked_lookup(llt, 0b00011110), (72, 8));
        assert_eq!(masked_lookup(llt, 0b01011110), (74, 8));
        assert_eq!(masked_lookup(llt, 0b11111101), (143, 8));
        assert_eq!(masked_lookup(llt, 0b000010011), (144, 9));
        assert_eq!(masked_lookup(llt, 0b111111111), (255, 9));
        assert_eq!(masked_lookup(llt, 0b00000000), (256, 7));
        assert_eq!(masked_lookup(llt, 0b1110100), (279, 7));
        assert_eq!(masked_lookup(llt, 0b00000011), (280, 8));
        assert_eq!(masked_lookup(llt, 0b11100011), (287, 8));

        assert_eq!(masked_lookup(dt, 0), (0, 5));
        assert_eq!(masked_lookup(dt, 20), (5, 5));
    }

    // Only run this test with alloc enabled as it uses a larger buffer.
    #[cfg(feature = "with-alloc")]
    fn check_result(input: &[u8], expected_status: TINFLStatus, expected_state: State, zlib: bool) {
        let mut r = DecompressorOxide::default();
        let mut output_buf = vec![0; 1024 * 32];
        let flags = if zlib {
            inflate_flags::TINFL_FLAG_PARSE_ZLIB_HEADER
        } else {
            0
        } | TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF
            | TINFL_FLAG_HAS_MORE_INPUT;
        let (d_status, _in_bytes, _out_bytes) =
            decompress(&mut r, input, &mut output_buf, 0, flags);
        assert_eq!(expected_status, d_status);
        assert_eq!(expected_state, r.state);
    }

    #[cfg(feature = "with-alloc")]
    #[test]
    fn bogus_input() {
        use self::check_result as cr;
        const F: TINFLStatus = TINFLStatus::Failed;
        const OK: TINFLStatus = TINFLStatus::Done;
        // Bad CM.
        cr(&[0x77, 0x85], F, State::BadZlibHeader, true);
        // Bad window size (but check is correct).
        cr(&[0x88, 0x98], F, State::BadZlibHeader, true);
        // Bad check bits.
        cr(&[0x78, 0x98], F, State::BadZlibHeader, true);

        // Too many code lengths. (From inflate library issues)
        cr(
            b"M\xff\xffM*\xad\xad\xad\xad\xad\xad\xad\xcd\xcd\xcdM",
            F,
            State::BadDistOrLiteralTableLength,
            false,
        );

        // Bad CLEN (also from inflate library issues)
        cr(
            b"\xdd\xff\xff*M\x94ffffffffff",
            F,
            State::BadDistOrLiteralTableLength,
            false,
        );

        // Port of inflate coverage tests from zlib-ng
        // https://github.com/Dead2/zlib-ng/blob/develop/test/infcover.c
        let c = |a, b, c| cr(a, b, c, false);

        // Invalid uncompressed/raw block length.
        c(&[0, 0, 0, 0, 0], F, State::BadRawLength);
        // Ok empty uncompressed block.
        c(&[3, 0], OK, State::DoneForever);
        // Invalid block type.
        c(&[6], F, State::BlockTypeUnexpected);
        // Ok uncompressed block.
        c(&[1, 1, 0, 0xfe, 0xff, 0], OK, State::DoneForever);
        // Too many litlens, we handle this later than zlib, so this test won't
        // give the same result.
        //        c(&[0xfc, 0, 0], F, State::BadTotalSymbols);
        // Invalid set of code lengths - TODO Check if this is the correct error for this.
        c(&[4, 0, 0xfe, 0xff], F, State::BadTotalSymbols);
        // Invalid repeat in list of code lengths.
        // (Try to repeat a non-existent code.)
        c(&[4, 0, 0x24, 0x49, 0], F, State::BadCodeSizeDistPrevLookup);
        // Missing end of block code (should we have a separate error for this?) - fails on further input
        //    c(&[4, 0, 0x24, 0xe9, 0xff, 0x6d], F, State::BadTotalSymbols);
        // Invalid set of literals/lengths
        c(
            &[
                4, 0x80, 0x49, 0x92, 0x24, 0x49, 0x92, 0x24, 0x71, 0xff, 0xff, 0x93, 0x11, 0,
            ],
            F,
            State::BadTotalSymbols,
        );
        // Invalid set of distances _ needsmoreinput
        // c(&[4, 0x80, 0x49, 0x92, 0x24, 0x49, 0x92, 0x24, 0x0f, 0xb4, 0xff, 0xff, 0xc3, 0x84], F, State::BadTotalSymbols);
        // Invalid distance code
        c(&[2, 0x7e, 0xff, 0xff], F, State::InvalidDist);

        // Distance refers to position before the start
        c(
            &[0x0c, 0xc0, 0x81, 0, 0, 0, 0, 0, 0x90, 0xff, 0x6b, 0x4, 0],
            F,
            State::DistanceOutOfBounds,
        );

        // Trailer
        // Bad gzip trailer checksum GZip header not handled by miniz_oxide
        //cr(&[0x1f, 0x8b, 0x08 ,0 ,0 ,0 ,0 ,0 ,0 ,0 ,0x03, 0, 0, 0, 0, 0x01], F, State::BadCRC, false)
        // Bad gzip trailer length
        //cr(&[0x1f, 0x8b, 0x08 ,0 ,0 ,0 ,0 ,0 ,0 ,0 ,0x03, 0, 0, 0, 0, 0, 0, 0, 0, 0x01], F, State::BadCRC, false)
    }

    #[test]
    fn empty_output_buffer_non_wrapping() {
        let encoded = [
            120, 156, 243, 72, 205, 201, 201, 215, 81, 168, 202, 201, 76, 82, 4, 0, 27, 101, 4, 19,
        ];
        let flags = TINFL_FLAG_COMPUTE_ADLER32
            | TINFL_FLAG_PARSE_ZLIB_HEADER
            | TINFL_FLAG_USING_NON_WRAPPING_OUTPUT_BUF;
        let mut r = DecompressorOxide::new();
        let mut output_buf: [u8; 0] = [];
        // Check that we handle an empty buffer properly and not panicking.
        // https://github.com/Frommi/miniz_oxide/issues/23
        let res = decompress(&mut r, &encoded, &mut output_buf, 0, flags);
        assert!(res == (TINFLStatus::HasMoreOutput, 4, 0));
    }

    #[test]
    fn empty_output_buffer_wrapping() {
        let encoded = [
            0x73, 0x49, 0x4d, 0xcb, 0x49, 0x2c, 0x49, 0x55, 0x00, 0x11, 0x00,
        ];
        let flags = TINFL_FLAG_COMPUTE_ADLER32;
        let mut r = DecompressorOxide::new();
        let mut output_buf: [u8; 0] = [];
        // Check that we handle an empty buffer properly and not panicking.
        // https://github.com/Frommi/miniz_oxide/issues/23
        let res = decompress(&mut r, &encoded, &mut output_buf, 0, flags);
        assert!(res == (TINFLStatus::HasMoreOutput, 2, 0));
    }

    #[test]
    fn dist_extra_bits() {
        use self::num_extra_bits_for_distance_code;
        // Number of extra bits for each distance code.
        const DIST_EXTRA: [u8; 29] = [
            0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12,
            12, 13,
        ];

        for (i, &dist) in DIST_EXTRA.iter().enumerate() {
            assert_eq!(dist, num_extra_bits_for_distance_code(i as u8));
        }
    }

    #[test]
    fn check_tree() {
        let mut r = DecompressorOxide::new();
        let mut l = LocalVars {
            bit_buf: 0,
            num_bits: 0,
            dist: 0,
            counter: 0,
            num_extra: 0,
        };

        r.code_size_huffman[0] = 1;
        r.code_size_huffman[1] = 1;
        //r.code_size_huffman[2] = 3;
        //r.code_size_huffman[3] = 3;
        //r.code_size_huffman[1] = 4;
        r.block_type = HUFFLEN_TABLE as u8;
        r.table_sizes[HUFFLEN_TABLE] = 4;
        let res = init_tree(&mut r, &mut l).unwrap();

        let status = match res {
            Action::Jump(s) => s,
            _ => {
                //println!("issue");
                return;
            }
        };
        //println!("status {:?}", status);
        assert!(status != BadTotalSymbols);
    }

    #[test]
    fn reverse_bits_lookup() {
        use super::reverse_bits;
        for i in 0..512 {
            assert_eq!(reverse_bits(i), i.reverse_bits());
        }
    }
}
