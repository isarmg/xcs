#[cfg(test)]
mod tests {
    use crate::write::{signed_len, unsigned_len};

    use super::*;
    use std;
    use std::io;

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
                low_bits_of_u64(i << 16 | i | (CONTINUATION_BIT as u64))
            );
        }
    }

    // Examples from the DWARF 4 standard, section 7.6, figure 22.
    #[test]
    fn test_read_unsigned() {
        let buf = [2u8];
        let mut readable = &buf[..];
        assert_eq!(
            2,
            read::unsigned(&mut readable).expect("Should read number")
        );

        let buf = [127u8];
        let mut readable = &buf[..];
        assert_eq!(
            127,
            read::unsigned(&mut readable).expect("Should read number")
        );

        let buf = [CONTINUATION_BIT, 1];
        let mut readable = &buf[..];
        assert_eq!(
            128,
            read::unsigned(&mut readable).expect("Should read number")
        );

        let buf = [1u8 | CONTINUATION_BIT, 1];
        let mut readable = &buf[..];
        assert_eq!(
            129,
            read::unsigned(&mut readable).expect("Should read number")
        );

        let buf = [2u8 | CONTINUATION_BIT, 1];
        let mut readable = &buf[..];
        assert_eq!(
            130,
            read::unsigned(&mut readable).expect("Should read number")
        );

        let buf = [57u8 | CONTINUATION_BIT, 100];
        let mut readable = &buf[..];
        assert_eq!(
            12857,
            read::unsigned(&mut readable).expect("Should read number")
        );
    }

    #[test]
    fn test_read_unsigned_thru_dyn_trait() {
        fn read(r: &mut dyn io::Read) -> u64 {
            read::unsigned(r).expect("Should read number")
        }

        let buf = [0u8];

        let mut readable = &buf[..];
        assert_eq!(0, read(&mut readable));

        let mut readable = io::Cursor::new(buf);
        assert_eq!(0, read(&mut readable));
    }

    // Examples from the DWARF 4 standard, section 7.6, figure 23.
    #[test]
    fn test_read_signed() {
        let buf = [2u8];
        let mut readable = &buf[..];
        assert_eq!(2, read::signed(&mut readable).expect("Should read number"));

        let buf = [0x7eu8];
        let mut readable = &buf[..];
        assert_eq!(-2, read::signed(&mut readable).expect("Should read number"));

        let buf = [127u8 | CONTINUATION_BIT, 0];
        let mut readable = &buf[..];
        assert_eq!(
            127,
            read::signed(&mut readable).expect("Should read number")
        );

        let buf = [1u8 | CONTINUATION_BIT, 0x7f];
        let mut readable = &buf[..];
        assert_eq!(
            -127,
            read::signed(&mut readable).expect("Should read number")
        );

        let buf = [CONTINUATION_BIT, 1];
        let mut readable = &buf[..];
        assert_eq!(
            128,
            read::signed(&mut readable).expect("Should read number")
        );

        let buf = [CONTINUATION_BIT, 0x7f];
        let mut readable = &buf[..];
        assert_eq!(
            -128,
            read::signed(&mut readable).expect("Should read number")
        );

        let buf = [1u8 | CONTINUATION_BIT, 1];
        let mut readable = &buf[..];
        assert_eq!(
            129,
            read::signed(&mut readable).expect("Should read number")
        );

        let buf = [0x7fu8 | CONTINUATION_BIT, 0x7e];
        let mut readable = &buf[..];
        assert_eq!(
            -129,
            read::signed(&mut readable).expect("Should read number")
        );
    }

    #[test]
    fn test_read_signed_thru_dyn_trait() {
        fn read(r: &mut dyn io::Read) -> i64 {
            read::signed(r).expect("Should read number")
        }

        let buf = [0u8];

        let mut readable = &buf[..];
        assert_eq!(0, read(&mut readable));

        let mut readable = io::Cursor::new(buf);
        assert_eq!(0, read(&mut readable));
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
        let mut readable = &buf[..];
        assert_eq!(
            -0x4000000000000000,
            read::signed(&mut readable).expect("Should read number")
        );
    }

    #[test]
    fn test_read_unsigned_not_enough_data() {
        let buf = [CONTINUATION_BIT];
        let mut readable = &buf[..];
        match read::unsigned(&mut readable) {
            Err(read::Error::IoError(e)) => assert_eq!(e.kind(), io::ErrorKind::UnexpectedEof),
            otherwise => panic!("Unexpected: {:?}", otherwise),
        }
    }

    #[test]
    fn test_read_signed_not_enough_data() {
        let buf = [CONTINUATION_BIT];
        let mut readable = &buf[..];
        match read::signed(&mut readable) {
            Err(read::Error::IoError(e)) => assert_eq!(e.kind(), io::ErrorKind::UnexpectedEof),
            otherwise => panic!("Unexpected: {:?}", otherwise),
        }
    }

    #[test]
    fn test_write_unsigned_not_enough_space() {
        let mut buf = [0; 1];
        let mut writable = &mut buf[..];
        match write::unsigned(&mut writable, 128) {
            Err(e) => assert_eq!(e.kind(), io::ErrorKind::WriteZero),
            otherwise => panic!("Unexpected: {:?}", otherwise),
        }
    }

    #[test]
    fn test_write_signed_not_enough_space() {
        let mut buf = [0; 1];
        let mut writable = &mut buf[..];
        match write::signed(&mut writable, 128) {
            Err(e) => assert_eq!(e.kind(), io::ErrorKind::WriteZero),
            otherwise => panic!("Unexpected: {:?}", otherwise),
        }
    }

    #[test]
    fn test_write_unsigned_thru_dyn_trait() {
        fn write(w: &mut dyn io::Write, val: u64) -> usize {
            write::unsigned(w, val).expect("Should write number")
        }
        let mut buf = [0u8; 1];

        let mut writable = &mut buf[..];
        assert_eq!(write(&mut writable, 0), 1);
        assert_eq!(buf[0], 0);

        let mut writable = Vec::from(&buf[..]);
        assert_eq!(write(&mut writable, 0), 1);
        assert_eq!(buf[0], 0);
    }

    #[test]
    fn test_write_signed_thru_dyn_trait() {
        fn write(w: &mut dyn io::Write, val: i64) -> usize {
            write::signed(w, val).expect("Should write number")
        }
        let mut buf = [0u8; 1];

        let mut writable = &mut buf[..];
        assert_eq!(write(&mut writable, 0), 1);
        assert_eq!(buf[0], 0);

        let mut writable = Vec::from(&buf[..]);
        assert_eq!(write(&mut writable, 0), 1);
        assert_eq!(buf[0], 0);
    }

    #[test]
    fn dogfood_signed() {
        fn inner(i: i64) {
            let mut buf = [0u8; 1024];

            {
                let mut writable = &mut buf[..];
                write::signed(&mut writable, i).expect("Should write signed number");
            }

            let mut readable = &buf[..];
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

            let mut readable = &buf[..];
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
        let mut readable = &buf[..];
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
        let mut readable = &buf[..];
        assert!(read::signed(&mut readable).is_err());
    }

    #[test]
    fn test_read_multiple() {
        let buf = [2u8 | CONTINUATION_BIT, 1u8, 1u8];

        let mut readable = &buf[..];
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
    fn test_read_multiple_with_overflow() {
        let buf = [
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b0111_1111, // Overflow!
            0b1110_0100,
            0b1110_0000,
            0b0000_0010, // 45156
        ];
        let mut readable = &buf[..];

        assert!(if let read::Error::Overflow =
            read::unsigned(&mut readable).expect_err("Should fail with Error::Overflow")
        {
            true
        } else {
            false
        });
        assert_eq!(
            read::unsigned(&mut readable).expect("Should succeed with correct value"),
            45156
        );
    }

    #[test]
    fn test_write_unsigned_len() {
        assert_eq!(unsigned_len(0), 1);
        assert_eq!(unsigned_len(64), 1);
        assert_eq!(unsigned_len(127), 1);
        assert_eq!(unsigned_len(128), 2);
        assert_eq!(unsigned_len((1 << 42) - 1), 6);
        assert_eq!(unsigned_len(u64::MAX), 10);
    }

    #[test]
    fn test_write_signed_len() {
        assert_eq!(signed_len(0), 1);
        assert_eq!(signed_len(32), 1);
        assert_eq!(signed_len(64), 2);
        assert_eq!(signed_len(-1), 1);
        assert_eq!(signed_len(-32), 1);
        assert_eq!(signed_len(i64::MAX), 10);
    }
}
