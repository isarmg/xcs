#[cfg(all(feature = "nightly", test))]
mod benches {
    extern crate std;
    extern crate test;

    use super::{Parser, Receiver};

    use self::test::{black_box, Bencher};

    static UTF8_DEMO: &[u8] = include_bytes!("../../../tests/UTF-8-demo.txt");

    impl Receiver for () {
        fn codepoint(&mut self, c: char) {
            black_box(c);
        }

        fn invalid_sequence(&mut self) {}
    }

    #[bench]
    fn parse_bench_utf8_demo(b: &mut Bencher) {
        let mut parser = Parser::new();

        b.iter(|| {
            for byte in UTF8_DEMO {
                parser.advance(&mut (), *byte);
            }
        })
    }

    #[bench]
    fn std_string_parse_utf8(b: &mut Bencher) {
        b.iter(|| {
            for c in std::str::from_utf8(UTF8_DEMO).unwrap().chars() {
                black_box(c);
            }
        });
    }
}
