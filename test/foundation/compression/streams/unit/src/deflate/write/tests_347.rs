#[cfg(test)]
mod tests {
    use super::*;
    use crate::streams::Compression;
    use alloc::string::String;
    use alloc::vec::Vec;

    const STR: &str = "Hello World Hello World Hello World Hello World Hello World \
        Hello World Hello World Hello World Hello World Hello World \
        Hello World Hello World Hello World Hello World Hello World \
        Hello World Hello World Hello World Hello World Hello World \
        Hello World Hello World Hello World Hello World Hello World";

    // DeflateDecoder consumes one zlib archive and then returns 0 for subsequent writes, allowing any
    // additional data to be consumed by the caller.
    #[test]
    fn decode_extra_data() {
        let compressed = {
            let mut e = DeflateEncoder::new(Vec::new(), Compression::default());
            e.write_all(STR.as_ref()).unwrap();
            let mut b = e.finish().unwrap();
            b.push(b'x');
            b
        };

        let mut writer = Vec::new();
        let mut decoder = DeflateDecoder::new(writer);
        let mut consumed_bytes = 0;
        loop {
            let n = decoder.write(&compressed[consumed_bytes..]).unwrap();
            if n == 0 {
                break;
            }
            consumed_bytes += n;
        }
        writer = decoder.finish().unwrap();
        let actual = String::from_utf8(writer).expect("String parsing error");
        assert_eq!(actual, STR);
        assert_eq!(&compressed[consumed_bytes..], b"x");
    }
}
