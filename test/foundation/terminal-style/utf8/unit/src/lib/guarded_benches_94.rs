#[cfg(test)]
mod benches {
    extern crate std;

    use super::{Parser, Receiver};

    use self::std::hint::black_box;

    // Execute the retained benchmark bodies as stable smoke tests.
    struct Bencher;
    impl Bencher {
        fn iter(&mut self, mut body: impl FnMut()) {
            body();
        }
    }

    static UTF8_DEMO: &[u8] = include_bytes!("../../../tests/UTF-8-demo.txt");

    impl Receiver for () {
        fn codepoint(&mut self, c: char) {
            black_box(c);
        }

        fn invalid_sequence(&mut self) {}
    }

    #[test]
    fn parse_bench_utf8_demo() {
        let b = &mut Bencher;
        let mut parser = Parser::new();

        b.iter(|| {
            for byte in UTF8_DEMO {
                parser.advance(&mut (), *byte);
            }
        })
    }

    #[test]
    fn std_string_parse_utf8() {
        let b = &mut Bencher;
        b.iter(|| {
            for c in std::str::from_utf8(UTF8_DEMO).unwrap().chars() {
                black_box(c);
            }
        });
    }
}
