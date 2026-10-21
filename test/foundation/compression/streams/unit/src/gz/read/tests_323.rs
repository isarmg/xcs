#[cfg(test)]
mod tests {
    use crate::streams::io::{Cursor, ErrorKind, Read, Result, Write};
    use alloc::vec::Vec;

    use super::GzDecoder;

    //a cursor turning EOF into blocking errors
    #[derive(Debug)]
    pub struct BlockingCursor {
        pub cursor: Cursor<Vec<u8>>,
    }

    impl BlockingCursor {
        pub fn new() -> BlockingCursor {
            BlockingCursor {
                cursor: Cursor::new(Vec::new()),
            }
        }

        pub fn set_position(&mut self, pos: u64) {
            self.cursor.set_position(pos)
        }
    }

    impl Write for BlockingCursor {
        fn write(&mut self, buf: &[u8]) -> Result<usize> {
            self.cursor.write(buf)
        }
        fn flush(&mut self) -> Result<()> {
            self.cursor.flush()
        }
    }

    impl Read for BlockingCursor {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
            //use the cursor, except it turns eof into blocking error
            let r = self.cursor.read(buf);
            match r {
                Err(ref err) => {
                    if err.kind() == ErrorKind::UnexpectedEof {
                        return Err(ErrorKind::WouldBlock.into());
                    }
                }
                Ok(0) => {
                    //regular EOF turned into blocking error
                    return Err(ErrorKind::WouldBlock.into());
                }
                Ok(_n) => {}
            }
            r
        }
    }

    #[test]
    fn blocked_partial_header_read() {
        // this is a reader which receives data afterwards
        let mut r = BlockingCursor::new();
        let data = vec![1, 2, 3];

        match r.write_all(&data) {
            Ok(()) => {}
            _ => {
                panic!("Unexpected result for write_all");
            }
        }
        r.set_position(0);

        // this is unused except for the buffering
        let mut decoder = GzDecoder::new(r);
        let mut out = Vec::with_capacity(7);
        match decoder.read(&mut out) {
            Err(e) => {
                assert_eq!(e.kind(), ErrorKind::WouldBlock);
            }
            _ => {
                panic!("Unexpected result for decoder.read");
            }
        }
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
        assert!(corrupted.len() > 13);
        corrupted[12] ^= 0xFF;
        corrupted[13] ^= 0xFF;

        // Try to decode corrupted data
        let mut decoder = GzDecoder::new(Cursor::new(corrupted));
        let mut output = Vec::new();
        let _ = decoder.read_to_end(&mut output).unwrap_err();

        // Reset with valid payload and decode
        decoder.reset(Cursor::new(valid_compressed));
        output.clear();
        decoder.read_to_end(&mut output).unwrap();
        assert_eq!(output, valid_data);
    }
}
