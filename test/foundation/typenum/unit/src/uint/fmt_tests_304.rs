#[cfg(test)]
mod fmt_tests {
    use super::*;
    use crate::consts::*;
    use core::fmt::Write;

    struct LimitedString {
        len: usize,
        buffer: [u8; 64],
    }

    impl LimitedString {
        fn new() -> Self {
            Self {
                len: 0,
                buffer: [0u8; 64],
            }
        }

        fn as_str(&self) -> &str {
            core::str::from_utf8(&self.buffer[..self.len]).unwrap()
        }
    }

    impl core::fmt::Write for LimitedString {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            self.buffer[self.len..self.len + s.len()].copy_from_slice(s.as_bytes());
            self.len += s.len();
            Ok(())
        }
    }

    fn assert_binary_fmt<U: Unsigned + core::fmt::Binary>(expected: &str) {
        let mut s = LimitedString::new();
        write!(&mut s, "{:b}", U::default()).unwrap();
        assert_eq!(s.as_str(), expected);
    }

    #[test]
    fn binary() {
        assert_binary_fmt::<U0>("0");
        assert_binary_fmt::<U1>("1");
        assert_binary_fmt::<U2>("10");
        assert_binary_fmt::<U3>("11");
        assert_binary_fmt::<U4>("100");
        assert_binary_fmt::<U5>("101");
        assert_binary_fmt::<U6>("110");
        assert_binary_fmt::<U7>("111");
        assert_binary_fmt::<U8>("1000");
        assert_binary_fmt::<U9>("1001");
        assert_binary_fmt::<U10>("1010");

        assert_binary_fmt::<U2147483648>("10000000000000000000000000000000");
    }
}
