#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;

    const STR: &str = "Hello World Hello World Hello World Hello World Hello World \
                               Hello World Hello World Hello World Hello World Hello World \
                               Hello World Hello World Hello World Hello World Hello World \
                               Hello World Hello World Hello World Hello World Hello World \
                               Hello World Hello World Hello World Hello World Hello World";

    #[test]
    fn decode_writer_one_chunk() {
        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        e.write_all(STR.as_ref()).unwrap();
        let bytes = e.finish().unwrap();

        let mut writer = Vec::new();
        let mut decoder = GzDecoder::new(writer);
        let n = decoder.write(&bytes[..]).unwrap();
        decoder.write_all(&bytes[n..]).unwrap();
        decoder.try_finish().unwrap();
        writer = decoder.finish().unwrap();
        let return_string = String::from_utf8(writer).expect("String parsing error");
        assert_eq!(return_string, STR);
    }

    #[test]
    fn decode_writer_partial_header() {
        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        e.write_all(STR.as_ref()).unwrap();
        let bytes = e.finish().unwrap();

        let mut writer = Vec::new();
        let mut decoder = GzDecoder::new(writer);
        assert_eq!(decoder.write(&bytes[..5]).unwrap(), 5);
        let n = decoder.write(&bytes[5..]).unwrap();
        if n < bytes.len() - 5 {
            decoder.write_all(&bytes[n + 5..]).unwrap();
        }
        writer = decoder.finish().unwrap();
        let return_string = String::from_utf8(writer).expect("String parsing error");
        assert_eq!(return_string, STR);
    }

    #[test]
    fn decode_writer_partial_header_filename() {
        let filename = "test.txt";
        let mut e = GzBuilder::new()
            .filename(filename)
            .read(STR.as_bytes(), Compression::default());
        let mut bytes = Vec::new();
        e.read_to_end(&mut bytes).unwrap();

        let mut writer = Vec::new();
        let mut decoder = GzDecoder::new(writer);
        assert_eq!(decoder.write(&bytes[..12]).unwrap(), 12);
        let n = decoder.write(&bytes[12..]).unwrap();
        if n < bytes.len() - 12 {
            decoder.write_all(&bytes[n + 12..]).unwrap();
        }
        assert_eq!(
            decoder.header().unwrap().filename().unwrap(),
            filename.as_bytes()
        );
        writer = decoder.finish().unwrap();
        let return_string = String::from_utf8(writer).expect("String parsing error");
        assert_eq!(return_string, STR);
    }

    #[test]
    fn decode_writer_partial_header_comment() {
        let comment = "test comment";
        let mut e = GzBuilder::new()
            .comment(comment)
            .read(STR.as_bytes(), Compression::default());
        let mut bytes = Vec::new();
        e.read_to_end(&mut bytes).unwrap();

        let mut writer = Vec::new();
        let mut decoder = GzDecoder::new(writer);
        assert_eq!(decoder.write(&bytes[..12]).unwrap(), 12);
        let n = decoder.write(&bytes[12..]).unwrap();
        if n < bytes.len() - 12 {
            decoder.write_all(&bytes[n + 12..]).unwrap();
        }
        assert_eq!(
            decoder.header().unwrap().comment().unwrap(),
            comment.as_bytes()
        );
        writer = decoder.finish().unwrap();
        let return_string = String::from_utf8(writer).expect("String parsing error");
        assert_eq!(return_string, STR);
    }

    #[test]
    fn decode_writer_exact_header() {
        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        e.write_all(STR.as_ref()).unwrap();
        let bytes = e.finish().unwrap();

        let mut writer = Vec::new();
        let mut decoder = GzDecoder::new(writer);
        assert_eq!(decoder.write(&bytes[..10]).unwrap(), 10);
        decoder.write_all(&bytes[10..]).unwrap();
        writer = decoder.finish().unwrap();
        let return_string = String::from_utf8(writer).expect("String parsing error");
        assert_eq!(return_string, STR);
    }

    #[test]
    fn decode_writer_partial_crc() {
        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        e.write_all(STR.as_ref()).unwrap();
        let bytes = e.finish().unwrap();

        let mut writer = Vec::new();
        let mut decoder = GzDecoder::new(writer);
        let l = bytes.len() - 5;
        let n = decoder.write(&bytes[..l]).unwrap();
        decoder.write_all(&bytes[n..]).unwrap();
        writer = decoder.finish().unwrap();
        let return_string = String::from_utf8(writer).expect("String parsing error");
        assert_eq!(return_string, STR);
    }

    // Two or more gzip files concatenated form a multi-member gzip file. MultiGzDecoder will
    // concatenate the decoded contents of all members.
    #[test]
    fn decode_multi_writer() {
        let mut e = GzEncoder::new(Vec::new(), Compression::default());
        e.write_all(STR.as_ref()).unwrap();
        let bytes = e.finish().unwrap().repeat(2);

        let mut writer = Vec::new();
        let mut decoder = MultiGzDecoder::new(writer);
        let mut count = 0;
        while count < bytes.len() {
            let n = decoder.write(&bytes[count..]).unwrap();
            assert!(n != 0);
            count += n;
        }
        writer = decoder.finish().unwrap();
        let return_string = String::from_utf8(writer).expect("String parsing error");
        let expected = STR.repeat(2);
        assert_eq!(return_string, expected);
    }

    // GzDecoder consumes one gzip member and then returns 0 for subsequent writes, allowing any
    // additional data to be consumed by the caller.
    #[test]
    fn decode_extra_data() {
        let compressed = {
            let mut e = GzEncoder::new(Vec::new(), Compression::default());
            e.write_all(STR.as_ref()).unwrap();
            let mut b = e.finish().unwrap();
            b.push(b'x');
            b
        };

        let mut writer = Vec::new();
        let mut decoder = GzDecoder::new(writer);
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
