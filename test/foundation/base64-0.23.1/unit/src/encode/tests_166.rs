#[cfg(test)]
mod tests {
    use super::*;

    use crate::alphabet::PADDING_SYMBOL;
    use crate::{
        alphabet,
        engine::general_purpose::{GeneralPurpose, NO_PAD, STANDARD},
        tests::{assert_encode_sanity, random_config, random_engine},
    };
    use rand::distr::{Distribution, Uniform};
    use rand::{rngs, RngExt};
    use std::str;

    const URL_SAFE_NO_PAD_ENGINE: GeneralPurpose = GeneralPurpose::new(&alphabet::URL_SAFE, NO_PAD);

    #[test]
    fn encoded_size_correct_standard() {
        assert_encoded_length(0, 0, &STANDARD, true);

        assert_encoded_length(1, 4, &STANDARD, true);
        assert_encoded_length(2, 4, &STANDARD, true);
        assert_encoded_length(3, 4, &STANDARD, true);

        assert_encoded_length(4, 8, &STANDARD, true);
        assert_encoded_length(5, 8, &STANDARD, true);
        assert_encoded_length(6, 8, &STANDARD, true);

        assert_encoded_length(7, 12, &STANDARD, true);
        assert_encoded_length(8, 12, &STANDARD, true);
        assert_encoded_length(9, 12, &STANDARD, true);

        assert_encoded_length(54, 72, &STANDARD, true);

        assert_encoded_length(55, 76, &STANDARD, true);
        assert_encoded_length(56, 76, &STANDARD, true);
        assert_encoded_length(57, 76, &STANDARD, true);

        assert_encoded_length(58, 80, &STANDARD, true);
    }

    #[test]
    fn encoded_size_correct_no_pad() {
        assert_encoded_length(0, 0, &URL_SAFE_NO_PAD_ENGINE, false);

        assert_encoded_length(1, 2, &URL_SAFE_NO_PAD_ENGINE, false);
        assert_encoded_length(2, 3, &URL_SAFE_NO_PAD_ENGINE, false);
        assert_encoded_length(3, 4, &URL_SAFE_NO_PAD_ENGINE, false);

        assert_encoded_length(4, 6, &URL_SAFE_NO_PAD_ENGINE, false);
        assert_encoded_length(5, 7, &URL_SAFE_NO_PAD_ENGINE, false);
        assert_encoded_length(6, 8, &URL_SAFE_NO_PAD_ENGINE, false);

        assert_encoded_length(7, 10, &URL_SAFE_NO_PAD_ENGINE, false);
        assert_encoded_length(8, 11, &URL_SAFE_NO_PAD_ENGINE, false);
        assert_encoded_length(9, 12, &URL_SAFE_NO_PAD_ENGINE, false);

        assert_encoded_length(54, 72, &URL_SAFE_NO_PAD_ENGINE, false);

        assert_encoded_length(55, 74, &URL_SAFE_NO_PAD_ENGINE, false);
        assert_encoded_length(56, 75, &URL_SAFE_NO_PAD_ENGINE, false);
        assert_encoded_length(57, 76, &URL_SAFE_NO_PAD_ENGINE, false);

        assert_encoded_length(58, 78, &URL_SAFE_NO_PAD_ENGINE, false);
    }

    #[test]
    fn encoded_size_overflow() {
        assert_eq!(None, encoded_len(usize::MAX, true));
    }

    #[test]
    fn encode_engine_string_into_nonempty_buffer_doesnt_clobber_prefix() {
        let mut orig_data = Vec::new();
        let mut prefix = String::new();
        let mut encoded_data_no_prefix = String::new();
        let mut encoded_data_with_prefix = String::new();
        let mut decoded = Vec::new();

        let prefix_len_range = Uniform::new(0, 1000).unwrap();
        let input_len_range = Uniform::new(0, 1000).unwrap();

        let mut rng = rand::make_rng::<rngs::SmallRng>();

        for _ in 0..10_000 {
            orig_data.clear();
            prefix.clear();
            encoded_data_no_prefix.clear();
            encoded_data_with_prefix.clear();
            decoded.clear();

            let input_len = input_len_range.sample(&mut rng);

            for _ in 0..input_len {
                orig_data.push(rng.random());
            }

            let prefix_len = prefix_len_range.sample(&mut rng);
            for _ in 0..prefix_len {
                // getting convenient random single-byte printable chars that aren't base64 is
                // annoying
                prefix.push('#');
            }
            encoded_data_with_prefix.push_str(&prefix);

            let engine = random_engine(&mut rng);
            engine.encode_string(&orig_data, &mut encoded_data_no_prefix);
            engine.encode_string(&orig_data, &mut encoded_data_with_prefix);

            assert_eq!(
                encoded_data_no_prefix.len() + prefix_len,
                encoded_data_with_prefix.len()
            );
            assert_encode_sanity(&encoded_data_no_prefix, &engine, input_len);
            assert_encode_sanity(&encoded_data_with_prefix[prefix_len..], &engine, input_len);

            // append plain encode onto prefix
            prefix.push_str(&encoded_data_no_prefix);

            assert_eq!(prefix, encoded_data_with_prefix);

            engine
                .decode_vec(&encoded_data_no_prefix, &mut decoded)
                .unwrap();
            assert_eq!(orig_data, decoded);
        }
    }

    #[test]
    fn encode_engine_slice_into_nonempty_buffer_doesnt_clobber_suffix() {
        let mut orig_data = Vec::new();
        let mut encoded_data = Vec::new();
        let mut encoded_data_original_state = Vec::new();
        let mut decoded = Vec::new();

        let input_len_range = Uniform::new(0, 1000).unwrap();

        let mut rng = rand::make_rng::<rngs::SmallRng>();

        for _ in 0..10_000 {
            orig_data.clear();
            encoded_data.clear();
            encoded_data_original_state.clear();
            decoded.clear();

            let input_len = input_len_range.sample(&mut rng);

            for _ in 0..input_len {
                orig_data.push(rng.random());
            }

            // plenty of existing garbage in the encoded buffer
            for _ in 0..10 * input_len {
                encoded_data.push(rng.random());
            }

            encoded_data_original_state.extend_from_slice(&encoded_data);

            let engine = random_engine(&mut rng);

            let encoded_size = encoded_len(input_len, engine.config().encode_padding()).unwrap();

            assert_eq!(
                encoded_size,
                engine.encode_slice(&orig_data, &mut encoded_data).unwrap()
            );

            assert_encode_sanity(
                str::from_utf8(&encoded_data[0..encoded_size]).unwrap(),
                &engine,
                input_len,
            );

            assert_eq!(
                &encoded_data[encoded_size..],
                &encoded_data_original_state[encoded_size..]
            );

            engine
                .decode_vec(&encoded_data[0..encoded_size], &mut decoded)
                .unwrap();
            assert_eq!(orig_data, decoded);
        }
    }

    #[test]
    fn encode_to_slice_random_valid_utf8() {
        let mut input = Vec::new();
        let mut output = Vec::new();

        let input_len_range = Uniform::new(0, 1000).unwrap();

        let mut rng = rand::make_rng::<rngs::SmallRng>();

        for _ in 0..10_000 {
            input.clear();
            output.clear();

            let input_len = input_len_range.sample(&mut rng);

            for _ in 0..input_len {
                input.push(rng.random());
            }

            let config = random_config(&mut rng);
            let engine = random_engine(&mut rng);

            // fill up the output buffer with garbage
            let encoded_size = encoded_len(input_len, config.encode_padding()).unwrap();
            for _ in 0..encoded_size {
                output.push(rng.random());
            }

            let orig_output_buf = output.clone();

            let bytes_written = engine.internal_encode(&input, &mut output);

            // make sure the part beyond bytes_written is the same garbage it was before
            assert_eq!(orig_output_buf[bytes_written..], output[bytes_written..]);

            // make sure the encoded bytes are UTF-8
            let _ = str::from_utf8(&output[0..bytes_written]).unwrap();
        }
    }

    #[test]
    fn encode_with_padding_random_valid_utf8() {
        let mut input = Vec::new();
        let mut output = Vec::new();

        let input_len_range = Uniform::new(0, 1000).unwrap();

        let mut rng = rand::make_rng::<rngs::SmallRng>();

        for _ in 0..10_000 {
            input.clear();
            output.clear();

            let input_len = input_len_range.sample(&mut rng);

            for _ in 0..input_len {
                input.push(rng.random());
            }

            let engine = random_engine(&mut rng);

            // fill up the output buffer with garbage
            let encoded_size = encoded_len(input_len, engine.config().encode_padding()).unwrap();
            for _ in 0..encoded_size + 1000 {
                output.push(rng.random());
            }

            let orig_output_buf = output.clone();

            encode_with_padding(&input, &mut output[0..encoded_size], &engine, encoded_size);

            // make sure the part beyond b64 is the same garbage it was before
            assert_eq!(orig_output_buf[encoded_size..], output[encoded_size..]);

            // make sure the encoded bytes are UTF-8
            let _ = str::from_utf8(&output[0..encoded_size]).unwrap();
        }
    }

    #[test]
    fn add_padding_random_valid_utf8() {
        let mut output = Vec::new();

        let mut rng = rand::make_rng::<rngs::SmallRng>();

        // cover our bases for length % 4
        for unpadded_output_len in 0..20 {
            output.clear();

            // fill output with random
            for _ in 0..100 {
                output.push(rng.random());
            }

            let orig_output_buf = output.clone();

            let bytes_written = add_padding(unpadded_output_len, PADDING_SYMBOL, &mut output);

            // make sure the part beyond bytes_written is the same garbage it was before
            assert_eq!(orig_output_buf[bytes_written..], output[bytes_written..]);

            // make sure the encoded bytes are UTF-8
            let _ = str::from_utf8(&output[0..bytes_written]).unwrap();
        }
    }

    fn assert_encoded_length<E: Engine>(
        input_len: usize,
        enc_len: usize,
        engine: &E,
        padded: bool,
    ) {
        assert_eq!(enc_len, encoded_len(input_len, padded).unwrap());

        let mut bytes: Vec<u8> = Vec::new();
        let mut rng = rand::make_rng::<rngs::SmallRng>();

        for _ in 0..input_len {
            bytes.push(rng.random());
        }

        let encoded = engine.encode(&bytes);
        assert_encode_sanity(&encoded, engine, input_len);

        assert_eq!(enc_len, encoded.len());
    }

    #[test]
    fn encode_imap() {
        assert_eq!(
            &GeneralPurpose::new(&alphabet::IMAP_MUTF7, NO_PAD).encode(b"\xFB\xFF"),
            &GeneralPurpose::new(&alphabet::STANDARD, NO_PAD)
                .encode(b"\xFB\xFF")
                .replace('/', ",")
        );
    }
}
