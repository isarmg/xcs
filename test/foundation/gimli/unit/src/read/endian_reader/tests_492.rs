#[cfg(test)]
mod tests {
    use super::*;
    use crate::endianity::NativeEndian;
    use crate::read::Reader;

    fn native_reader<T: CloneStableDeref<Target = [u8]> + Debug>(
        bytes: T,
    ) -> EndianReader<NativeEndian, T> {
        EndianReader::new(bytes, NativeEndian)
    }

    const BUF: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 0];

    #[test]
    fn test_reader_split() {
        let mut reader = native_reader(BUF);
        let left = reader.split(3).unwrap();
        assert_eq!(left, native_reader(&BUF[..3]));
        assert_eq!(reader, native_reader(&BUF[3..]));
    }

    #[test]
    fn test_reader_split_out_of_bounds() {
        let mut reader = native_reader(BUF);
        assert!(reader.split(30).is_err());
    }

    #[test]
    fn bytes_and_len_and_range_and_eq() {
        let reader = native_reader(BUF);
        assert_eq!(reader.len(), BUF.len());
        assert_eq!(reader.bytes(), BUF);
        assert_eq!(reader, native_reader(BUF));

        let range = reader.range(2..8);
        let buf_range = &BUF[2..8];
        assert_eq!(range.len(), buf_range.len());
        assert_eq!(range.bytes(), buf_range);
        assert_ne!(range, native_reader(BUF));
        assert_eq!(range, native_reader(buf_range));

        let range_from = range.range_from(1..);
        let buf_range_from = &buf_range[1..];
        assert_eq!(range_from.len(), buf_range_from.len());
        assert_eq!(range_from.bytes(), buf_range_from);
        assert_ne!(range_from, native_reader(BUF));
        assert_eq!(range_from, native_reader(buf_range_from));

        let range_to = range_from.range_to(..4);
        let buf_range_to = &buf_range_from[..4];
        assert_eq!(range_to.len(), buf_range_to.len());
        assert_eq!(range_to.bytes(), buf_range_to);
        assert_ne!(range_to, native_reader(BUF));
        assert_eq!(range_to, native_reader(buf_range_to));
    }

    #[test]
    fn find() {
        let mut reader = native_reader(BUF);
        reader.skip(2).unwrap();
        assert_eq!(
            reader.find(5),
            Ok(BUF[2..].iter().position(|x| *x == 5).unwrap())
        );
    }

    #[test]
    fn indexing() {
        let mut reader = native_reader(BUF);
        reader.skip(2).unwrap();
        assert_eq!(reader[0], BUF[2]);
    }

    #[test]
    #[should_panic]
    fn indexing_out_of_bounds() {
        let mut reader = native_reader(BUF);
        reader.skip(2).unwrap();
        let _ = reader[900];
    }

    #[test]
    fn endian() {
        let reader = native_reader(BUF);
        assert_eq!(reader.endian(), NativeEndian);
    }

    #[test]
    fn empty() {
        let mut reader = native_reader(BUF);
        assert!(!reader.is_empty());
        reader.empty();
        assert!(reader.is_empty());
        assert!(reader.bytes().is_empty());
    }

    #[test]
    fn truncate() {
        let reader = native_reader(BUF);
        let mut reader = reader.range(2..8);
        reader.truncate(2).unwrap();
        assert_eq!(reader.bytes(), &BUF[2..4]);
    }

    #[test]
    fn offset_from() {
        let reader = native_reader(BUF);
        let sub = reader.range(2..8);
        assert_eq!(sub.offset_from(&reader), 2);
    }

    #[test]
    fn skip() {
        let mut reader = native_reader(BUF);
        reader.skip(2).unwrap();
        assert_eq!(reader.bytes(), &BUF[2..]);
    }

    #[test]
    fn to_slice() {
        assert_eq!(
            native_reader(BUF).range(2..5).to_slice(),
            Ok(Cow::from(&BUF[2..5]))
        );
    }

    #[test]
    fn to_string_ok() {
        let buf = b"hello, world!";
        let reader = native_reader(&buf[..]);
        let reader = reader.range_from(7..);
        assert_eq!(reader.to_string(), Ok(Cow::from("world!")));
    }

    // The rocket emoji (🚀 = [0xf0, 0x9f, 0x9a, 0x80]) but rotated left by one
    // to make it invalid UTF-8.
    const BAD_UTF8: &[u8] = &[0x9f, 0x9a, 0x80, 0xf0];

    #[test]
    fn to_string_err() {
        let reader = native_reader(BAD_UTF8);
        assert!(reader.to_string().is_err());
    }

    #[test]
    fn to_string_lossy() {
        let reader = native_reader(BAD_UTF8);
        assert_eq!(reader.to_string_lossy(), Ok(Cow::from("����")));
    }

    #[test]
    fn read_u8_array() {
        let mut reader = native_reader(BAD_UTF8);
        reader.skip(1).unwrap();
        let arr: [u8; 2] = reader.read_u8_array().unwrap();
        assert_eq!(arr, &BAD_UTF8[1..3]);
        assert_eq!(reader.bytes(), &BAD_UTF8[3..]);
    }
}
