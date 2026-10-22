#[cfg(test)]
mod test {
    use super::*;
    // Upstream property generator unavailable in the maintained offline graph.
    #[cfg(any())]
    use proptest::prelude::*;
    use std::io::Write as _;

    #[cfg(any())]
    proptest! {
        #[test]
        #[cfg_attr(miri, ignore)]  // See https://github.com/AltSysrq/proptest/issues/253
        fn write_all_no_escapes(s in "\\PC*") {
            let buffer = Vec::new();
            let mut stream = StripStream::new(buffer);
            stream.write_all(s.as_bytes()).unwrap();
            let buffer = stream.into_inner();
            let actual = std::str::from_utf8(buffer.as_ref()).unwrap();
            assert_eq!(s, actual);
        }

        #[test]
        #[cfg_attr(miri, ignore)]  // See https://github.com/AltSysrq/proptest/issues/253
        fn write_byte_no_escapes(s in "\\PC*") {
            let buffer = Vec::new();
            let mut stream = StripStream::new(buffer);
            for byte in s.as_bytes() {
                stream.write_all(&[*byte]).unwrap();
            }
            let buffer = stream.into_inner();
            let actual = std::str::from_utf8(buffer.as_ref()).unwrap();
            assert_eq!(s, actual);
        }

        #[test]
        #[cfg_attr(miri, ignore)]  // See https://github.com/AltSysrq/proptest/issues/253
        fn write_all_random(s in any::<Vec<u8>>()) {
            let buffer = Vec::new();
            let mut stream = StripStream::new(buffer);
            stream.write_all(s.as_slice()).unwrap();
            let buffer = stream.into_inner();
            if let Ok(actual) = std::str::from_utf8(buffer.as_ref()) {
                for char in actual.chars() {
                    assert!(!char.is_ascii() || !char.is_control() || char.is_ascii_whitespace(), "{:?} -> {:?}: {:?}", String::from_utf8_lossy(&s), actual, char);
                }
            }
        }

        #[test]
        #[cfg_attr(miri, ignore)]  // See https://github.com/AltSysrq/proptest/issues/253
        fn write_byte_random(s in any::<Vec<u8>>()) {
            let buffer = Vec::new();
            let mut stream = StripStream::new(buffer);
            for byte in s.as_slice() {
                stream.write_all(&[*byte]).unwrap();
            }
            let buffer = stream.into_inner();
            if let Ok(actual) = std::str::from_utf8(buffer.as_ref()) {
                for char in actual.chars() {
                    assert!(!char.is_ascii() || !char.is_control() || char.is_ascii_whitespace(), "{:?} -> {:?}: {:?}", String::from_utf8_lossy(&s), actual, char);
                }
            }
        }
    }
}
