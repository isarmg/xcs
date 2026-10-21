#[cfg(test)]
#[cfg(all(feature = "read", feature = "write"))]
mod tests {
    use super::{low_bits_of_byte, low_bits_of_u64, read, write, CONTINUATION_BIT};
    use crate::endianity::NativeEndian;
    use crate::read::{EndianSlice, Error, ReaderOffsetId};

    trait ResultExt {
        fn map_eof(self, input: &[u8]) -> Self;
    }

    impl<T> ResultExt for Result<T, Error> {
        fn map_eof(self, input: &[u8]) -> Self {
            match self {
                Err(Error::UnexpectedEof(id)) => {
                    let id = ReaderOffsetId(id.0 - input.as_ptr() as u64);
                    Err(Error::UnexpectedEof(id))
                }
                r => r,
            }
        }
    }

    #[test]
    fn test_low_bits_of_byte() {
        for i in 0..127 {
            assert_eq!(i, low_bits_of_byte(i));
            assert_eq!(i, low_bits_of_byte(i | CONTINUATION_BIT));
        }
    }

    #[test]
    fn test_low_bits_of_u64() {
        for i in 0u64..127 {
            assert_eq!(i as u8, low_bits_of_u64(1 << 16 | i));
            assert_eq!(
                i as u8,
                low_bits_of_u64(i << 16 | i | (u64::from(CONTINUATION_BIT)))
            );
        }
    }

    // Examples from the DWARF 4 standard, section 7.6, figure 22.
    #[test]
    fn test_read_unsigned() {
        let buf = [2u8];
        let mut readable = EndianSlice::new(&buf[..], NativeEndian);
        assert_eq!(
            2,
            read::unsigned(&mut readable).expect("Should read number")
        );

        let buf = [127u8];
        let mut readable = EndianSlice::new(&buf[..], NativeEndian);
        assert_eq!(
            127,
            read::unsigned(&mut readable).expect("Should read number")
        );

        let buf = [CONTINUATION_BIT, 1];
        let mut readable = EndianSlice::new(&buf[..], NativeEndian);
        assert_eq!(
            128,
            read::unsigned(&mut readable).expect("Should read number")
        );

        let buf = [1u8 | CONTINUATION_BIT, 1];
        let mut readable = EndianSlice::new(&buf[..], NativeEndian);
        assert_eq!(
            129,
            read::unsigned(&mut readable).expect("Should read number")
        );

        let buf = [2u8 | CONTINUATION_BIT, 1];
        let mut readable = EndianSlice::new(&buf[..], NativeEndian);
        assert_eq!(
            130,
            read::unsigned(&mut readable).expect("Should read number")
        );

        let buf = [57u8 | CONTINUATION_BIT, 100];
        let mut readable = EndianSlice::new(&buf[..], NativeEndian);
        assert_eq!(
            12857,
            read::unsigned(&mut readable).expect("Should read number")
        );
    }

    // Examples from the DWARF 4 standard, section 7.6, figure 23.
    #[test]
    fn test_read_signed() {
        let buf = [2u8];
        let mut readable = EndianSlice::new(&buf[..], NativeEndian);
        assert_eq!(2, read::signed(&mut readable).expect("Should read number"));

        let buf = [0x7eu8];
        let mut readable = EndianSlice::new(&buf[..], NativeEndian);
        assert_eq!(-2, read::signed(&mut readable).expect("Should read number"));

        let buf = [127u8 | CONTINUATION_BIT, 0];
        let mut readable = EndianSlice::new(&buf[..], NativeEndian);
        assert_eq!(
            127,
            read::signed(&mut readable).expect("Should read number")
        );

        let buf = [1u8 | CONTINUATION_BIT, 0x7f];
        let mut readable = EndianSlice::new(&buf[..], NativeEndian);
        assert_eq!(
            -127,
            read::signed(&mut readable).expect("Should read number")
        );

        let buf = [CONTINUATION_BIT, 1];
        let mut readable = EndianSlice::new(&buf[..], NativeEndian);
        assert_eq!(
            128,
            read::signed(&mut readable).expect("Should read number")
        );

        let buf = [CONTINUATION_BIT, 0x7f];
        let mut readable = EndianSlice::new(&buf[..], NativeEndian);
        assert_eq!(
            -128,
            read::signed(&mut readable).expect("Should read number")
        );

        let buf = [1u8 | CONTINUATION_BIT, 1];
        let mut readable = EndianSlice::new(&buf[..], NativeEndian);
        assert_eq!(
            129,
            read::signed(&mut readable).expect("Should read number")
        );

        let buf = [0x7fu8 | CONTINUATION_BIT, 0x7e];
        let mut readable = EndianSlice::new(&buf[..], NativeEndian);
        assert_eq!(
            -129,
            read::signed(&mut readable).expect("Should read number")
        );
    }

    #[test]
    fn test_read_signed_63_bits() {
        let buf = [
            CONTINUATION_BIT,
            CONTINUATION_BIT,
            CONTINUATION_BIT,
            CONTINUATION_BIT,
            CONTINUATION_BIT,
            CONTINUATION_BIT,
            CONTINUATION_BIT,
            CONTINUATION_BIT,
            0x40,
        ];
        let mut readable = EndianSlice::new(&buf[..], NativeEndian);
        assert_eq!(
            -0x4000_0000_0000_0000,
            read::signed(&mut readable).expect("Should read number")
        );
    }

    #[test]
    fn test_read_unsigned_not_enough_data() {
        let buf = [CONTINUATION_BIT];
        let mut readable = EndianSlice::new(&buf[..], NativeEndian);
        assert_eq!(
            read::unsigned(&mut readable).map_eof(&buf),
            Err(Error::UnexpectedEof(ReaderOffsetId(1)))
        );
    }

    #[test]
    fn test_read_signed_not_enough_data() {
        let buf = [CONTINUATION_BIT];
        let mut readable = EndianSlice::new(&buf[..], NativeEndian);
        assert_eq!(
            read::signed(&mut readable).map_eof(&buf),
            Err(Error::UnexpectedEof(ReaderOffsetId(1)))
        );
    }

    #[test]
    fn test_write_unsigned_not_enough_space() {
        let mut buf = [0; 1];
        let mut writable = &mut buf[..];
        match write::unsigned(&mut writable, 128) {
            Err(e) => assert_eq!(e.kind(), std::io::ErrorKind::WriteZero),
            otherwise => panic!("Unexpected: {:?}", otherwise),
        }
    }

    #[test]
    fn test_write_signed_not_enough_space() {
        let mut buf = [0; 1];
        let mut writable = &mut buf[..];
        match write::signed(&mut writable, 128) {
            Err(e) => assert_eq!(e.kind(), std::io::ErrorKind::WriteZero),
            otherwise => panic!("Unexpected: {:?}", otherwise),
        }
    }

    #[test]
    fn dogfood_signed() {
        fn inner(i: i64) {
            let mut buf = [0u8; 1024];

            {
                let mut writable = &mut buf[..];
                write::signed(&mut writable, i).expect("Should write signed number");
            }

            let mut readable = EndianSlice::new(&buf[..], NativeEndian);
            let result = read::signed(&mut readable).expect("Should be able to read it back again");
            assert_eq!(i, result);
        }
        for i in -513..513 {
            inner(i);
        }
        inner(i64::MIN);
    }

    #[test]
    fn dogfood_unsigned() {
        for i in 0..1025 {
            let mut buf = [0u8; 1024];

            {
                let mut writable = &mut buf[..];
                write::unsigned(&mut writable, i).expect("Should write signed number");
            }

            let mut readable = EndianSlice::new(&buf[..], NativeEndian);
            let result =
                read::unsigned(&mut readable).expect("Should be able to read it back again");
            assert_eq!(i, result);
        }
    }

    #[test]
    fn test_read_unsigned_overflow() {
        let buf = [
            2u8 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            1,
        ];
        let mut readable = EndianSlice::new(&buf[..], NativeEndian);
        assert!(read::unsigned(&mut readable).is_err());
    }

    #[test]
    fn test_read_signed_overflow() {
        let buf = [
            2u8 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            1,
        ];
        let mut readable = EndianSlice::new(&buf[..], NativeEndian);
        assert!(read::signed(&mut readable).is_err());
    }

    #[test]
    fn test_read_multiple() {
        let buf = [2u8 | CONTINUATION_BIT, 1u8, 1u8];

        let mut readable = EndianSlice::new(&buf[..], NativeEndian);
        assert_eq!(
            read::unsigned(&mut readable).expect("Should read first number"),
            130u64
        );
        assert_eq!(
            read::unsigned(&mut readable).expect("Should read first number"),
            1u64
        );
    }

    #[test]
    fn test_read_u16() {
        for (buf, val) in [
            (&[2][..], 2),
            (&[0x7f][..], 0x7f),
            (&[0x80, 1][..], 0x80),
            (&[0x81, 1][..], 0x81),
            (&[0x82, 1][..], 0x82),
            (&[0xff, 0x7f][..], 0x3fff),
            (&[0x80, 0x80, 1][..], 0x4000),
            (&[0xff, 0xff, 1][..], 0x7fff),
            (&[0xff, 0xff, 3][..], 0xffff),
        ]
        .iter()
        {
            let mut readable = EndianSlice::new(buf, NativeEndian);
            assert_eq!(*val, read::u16(&mut readable).expect("Should read number"));
        }

        for buf in [
            &[0x80][..],
            &[0x80, 0x80][..],
            &[0x80, 0x80, 4][..],
            &[0x80, 0x80, 0x80, 3][..],
        ]
        .iter()
        {
            let mut readable = EndianSlice::new(buf, NativeEndian);
            assert!(read::u16(&mut readable).is_err(), "{:?}", buf);
        }
    }
}
