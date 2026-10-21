#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::{Base64, Base64Unpadded, Encoder, LineEnding, alphabet::Alphabet, test_vectors::*};

    #[test]
    fn encode_padded() {
        encode_test::<Base64>(PADDED_BIN, PADDED_BASE64, None);
    }

    #[test]
    fn encode_unpadded() {
        encode_test::<Base64Unpadded>(UNPADDED_BIN, UNPADDED_BASE64, None);
    }

    #[test]
    fn encode_multiline_padded() {
        encode_test::<Base64>(MULTILINE_PADDED_BIN, MULTILINE_PADDED_BASE64, Some(70));
    }

    #[test]
    fn encode_multiline_unpadded() {
        encode_test::<Base64Unpadded>(MULTILINE_UNPADDED_BIN, MULTILINE_UNPADDED_BASE64, Some(70));
    }

    #[test]
    fn no_trailing_newline_when_aligned() {
        let mut buffer = [0u8; 64];
        let mut encoder = Encoder::<Base64>::new_wrapped(&mut buffer, 64, LineEnding::LF).unwrap();
        encoder.encode(&[0u8; 48]).unwrap();

        // Ensure no newline character is present in this case
        assert_eq!(
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            encoder.finish().unwrap()
        );
    }

    /// Core functionality of an encoding test.
    fn encode_test<V: Alphabet>(input: &[u8], expected: &str, wrapped: Option<usize>) {
        let mut buffer = [0u8; 1024];

        for chunk_size in 1..input.len() {
            let mut encoder = match wrapped {
                Some(line_width) => {
                    Encoder::<V>::new_wrapped(&mut buffer, line_width, LineEnding::LF)
                }
                None => Encoder::<V>::new(&mut buffer),
            }
            .unwrap();

            for chunk in input.chunks(chunk_size) {
                encoder.encode(chunk).unwrap();
            }

            assert_eq!(expected, encoder.finish().unwrap());
        }
    }
}
