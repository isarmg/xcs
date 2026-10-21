#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::{Base64, Base64Unpadded, Decoder, alphabet::Alphabet, test_vectors::*};

    #[cfg(feature = "std")]
    use {alloc::vec::Vec, std::io::Read};

    #[test]
    fn decode_padded() {
        decode_test(PADDED_BIN, || {
            Decoder::<Base64>::new(PADDED_BASE64.as_bytes()).unwrap()
        })
    }

    #[test]
    fn decode_unpadded() {
        decode_test(UNPADDED_BIN, || {
            Decoder::<Base64Unpadded>::new(UNPADDED_BASE64.as_bytes()).unwrap()
        })
    }

    #[test]
    fn decode_multiline_padded() {
        decode_test(MULTILINE_PADDED_BIN, || {
            Decoder::<Base64>::new_wrapped(MULTILINE_PADDED_BASE64.as_bytes(), 70).unwrap()
        })
    }

    #[test]
    fn decode_multiline_unpadded() {
        decode_test(MULTILINE_UNPADDED_BIN, || {
            Decoder::<Base64Unpadded>::new_wrapped(MULTILINE_UNPADDED_BASE64.as_bytes(), 70)
                .unwrap()
        })
    }

    #[cfg(feature = "std")]
    #[test]
    fn read_multiline_padded() {
        let mut decoder =
            Decoder::<Base64>::new_wrapped(MULTILINE_PADDED_BASE64.as_bytes(), 70).unwrap();

        let mut buf = Vec::new();
        let len = decoder.read_to_end(&mut buf).unwrap();

        assert_eq!(len, MULTILINE_PADDED_BIN.len());
        assert_eq!(buf.as_slice(), MULTILINE_PADDED_BIN);
    }

    #[cfg(feature = "std")]
    #[test]
    fn decode_empty_at_end() {
        let mut decoder = Decoder::<Base64>::new(b"AAAA").unwrap();

        // Strip initial bytes
        let mut buf = vec![0u8; 3];
        assert_eq!(decoder.decode(&mut buf), Ok(&vec![0u8; 3][..]));

        // Now try to read nothing from the end
        let mut buf: Vec<u8> = vec![];

        assert_eq!(decoder.decode(&mut buf), Ok(&[][..]));
    }

    /// Core functionality of a decoding test
    #[allow(clippy::arithmetic_side_effects)]
    fn decode_test<'a, F, V>(expected: &[u8], f: F)
    where
        F: Fn() -> Decoder<'a, V>,
        V: Alphabet,
    {
        for chunk_size in 1..expected.len() {
            let mut decoder = f();
            let mut remaining_len = decoder.remaining_len();
            let mut buffer = [0u8; 1024];

            for chunk in expected.chunks(chunk_size) {
                assert!(!decoder.is_finished());
                let decoded = decoder.decode(&mut buffer[..chunk.len()]).unwrap();
                assert_eq!(chunk, decoded);

                let dlen = decoded.len();
                remaining_len -= dlen;
                assert_eq!(remaining_len, decoder.remaining_len());
            }

            assert!(decoder.is_finished());
            assert_eq!(decoder.remaining_len(), 0);
        }
    }
}
