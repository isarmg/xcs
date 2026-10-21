#[cfg(test)]
mod tests {
    use crate::{
        engine::Engine, tests::random_engine, write::encoder_string_writer::EncoderStringWriter,
    };
    use rand::Rng;
    use std::cmp;
    use std::io::Write;

    #[test]
    fn every_possible_split_of_input() {
        let mut rng = rand::thread_rng();
        let mut orig_data = Vec::<u8>::new();
        let mut normal_encoded = String::new();

        let size = 5_000;

        for i in 0..size {
            orig_data.clear();
            normal_encoded.clear();

            orig_data.resize(size, 0);
            rng.fill(&mut orig_data[..]);

            let engine = random_engine(&mut rng);
            engine.encode_string(&orig_data, &mut normal_encoded);

            let mut stream_encoder = EncoderStringWriter::new(&engine);
            // Write the first i bytes, then the rest
            stream_encoder.write_all(&orig_data[0..i]).unwrap();
            stream_encoder.write_all(&orig_data[i..]).unwrap();

            let stream_encoded = stream_encoder.into_inner();

            assert_eq!(normal_encoded, stream_encoded);
        }
    }
    #[test]
    fn incremental_writes() {
        let mut rng = rand::thread_rng();
        let mut orig_data = Vec::<u8>::new();
        let mut normal_encoded = String::new();

        let size = 5_000;

        for _ in 0..size {
            orig_data.clear();
            normal_encoded.clear();

            orig_data.resize(size, 0);
            rng.fill(&mut orig_data[..]);

            let engine = random_engine(&mut rng);
            engine.encode_string(&orig_data, &mut normal_encoded);

            let mut stream_encoder = EncoderStringWriter::new(&engine);
            // write small nibbles of data
            let mut offset = 0;
            while offset < size {
                let nibble_size = cmp::min(rng.gen_range(0..=64), size - offset);
                let len = stream_encoder
                    .write(&orig_data[offset..offset + nibble_size])
                    .unwrap();
                offset += len;
            }

            let stream_encoded = stream_encoder.into_inner();

            assert_eq!(normal_encoded, stream_encoded);
        }
    }
}
