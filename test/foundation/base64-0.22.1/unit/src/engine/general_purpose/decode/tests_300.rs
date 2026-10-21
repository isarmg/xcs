#[cfg(test)]
mod tests {
    use super::*;

    use crate::engine::general_purpose::STANDARD;

    #[test]
    fn decode_chunk_8_writes_only_6_bytes() {
        let input = b"Zm9vYmFy"; // "foobar"
        let mut output = [0_u8, 1, 2, 3, 4, 5, 6, 7];

        decode_chunk_8(&input[..], 0, &STANDARD.decode_table, &mut output).unwrap();
        assert_eq!(&vec![b'f', b'o', b'o', b'b', b'a', b'r', 6, 7], &output);
    }

    #[test]
    fn decode_chunk_4_writes_only_3_bytes() {
        let input = b"Zm9v"; // "foobar"
        let mut output = [0_u8, 1, 2, 3];

        decode_chunk_4(&input[..], 0, &STANDARD.decode_table, &mut output).unwrap();
        assert_eq!(&vec![b'f', b'o', b'o', 3], &output);
    }

    #[test]
    fn estimate_short_lengths() {
        for (range, decoded_len_estimate) in [
            (0..=0, 0),
            (1..=4, 3),
            (5..=8, 6),
            (9..=12, 9),
            (13..=16, 12),
            (17..=20, 15),
        ] {
            for encoded_len in range {
                let estimate = GeneralPurposeEstimate::new(encoded_len);
                assert_eq!(decoded_len_estimate, estimate.decoded_len_estimate());
            }
        }
    }

    #[test]
    fn estimate_via_u128_inflation() {
        // cover both ends of usize
        (0..1000)
            .chain(usize::MAX - 1000..=usize::MAX)
            .for_each(|encoded_len| {
                // inflate to 128 bit type to be able to safely use the easy formulas
                let len_128 = encoded_len as u128;

                let estimate = GeneralPurposeEstimate::new(encoded_len);
                assert_eq!(
                    (len_128 + 3) / 4 * 3,
                    estimate.conservative_decoded_len as u128
                );
            })
    }
}
