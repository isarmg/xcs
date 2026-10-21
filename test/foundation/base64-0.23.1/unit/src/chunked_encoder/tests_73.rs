#[cfg(test)]
pub mod tests {
    use crate::{
        alphabet::STANDARD,
        engine::general_purpose::{GeneralPurpose, GeneralPurposeConfig, PAD},
        tests::random_engine,
    };
    use rand::distr::{Distribution, Uniform};
    use rand::{rngs, RngExt};

    use super::*;

    #[test]
    fn chunked_encode_empty() {
        assert_eq!("", chunked_encode_str(&[], PAD));
    }

    #[test]
    fn chunked_encode_intermediate_fast_loop() {
        // > 8 bytes input, will enter the pretty fast loop
        assert_eq!("Zm9vYmFyYmF6cXV4", chunked_encode_str(b"foobarbazqux", PAD));
    }

    #[test]
    fn chunked_encode_fast_loop() {
        // > 32 bytes input, will enter the uber fast loop
        assert_eq!(
            "Zm9vYmFyYmF6cXV4cXV1eGNvcmdlZ3JhdWx0Z2FycGx5eg==",
            chunked_encode_str(b"foobarbazquxquuxcorgegraultgarplyz", PAD)
        );
    }

    #[test]
    fn chunked_encode_slow_loop_only() {
        // < 8 bytes input, slow loop only
        assert_eq!("Zm9vYmFy", chunked_encode_str(b"foobar", PAD));
    }

    #[test]
    fn chunked_encode_matches_normal_encode_random_string_sink() {
        let helper = StringSinkTestHelper;
        chunked_encode_matches_normal_encode_random(&helper);
    }

    pub fn chunked_encode_matches_normal_encode_random<S: SinkTestHelper>(sink_test_helper: &S) {
        let mut input_buf: Vec<u8> = Vec::new();
        let mut output_buf = String::new();
        let mut rng = rand::make_rng::<rngs::SmallRng>();
        let input_len_range = Uniform::new(1, 10_000).unwrap();

        for _ in 0..20_000 {
            input_buf.clear();
            output_buf.clear();

            let buf_len = input_len_range.sample(&mut rng);
            for _ in 0..buf_len {
                input_buf.push(rng.random());
            }

            let engine = random_engine(&mut rng);

            let chunk_encoded_string = sink_test_helper.encode_to_string(&engine, &input_buf);
            engine.encode_string(&input_buf, &mut output_buf);

            assert_eq!(output_buf, chunk_encoded_string, "input len={}", buf_len);
        }
    }

    fn chunked_encode_str(bytes: &[u8], config: GeneralPurposeConfig) -> String {
        let mut s = String::new();

        let mut sink = StringSink::new(&mut s);
        let engine = GeneralPurpose::new(&STANDARD, config);
        let encoder = ChunkedEncoder::new(&engine);
        encoder.encode(bytes, &mut sink).unwrap();

        s
    }

    // An abstraction around sinks so that we can have tests that easily to any sink implementation
    pub trait SinkTestHelper {
        fn encode_to_string<E: Engine>(&self, engine: &E, bytes: &[u8]) -> String;
    }

    struct StringSinkTestHelper;

    impl SinkTestHelper for StringSinkTestHelper {
        fn encode_to_string<E: Engine>(&self, engine: &E, bytes: &[u8]) -> String {
            let encoder = ChunkedEncoder::new(engine);
            let mut s = String::new();
            let mut sink = StringSink::new(&mut s);
            encoder.encode(bytes, &mut sink).unwrap();

            s
        }
    }
}
