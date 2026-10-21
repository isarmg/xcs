#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::Format;
    use crate::endianity::LittleEndian;
    use test_assembler::{Endian, Section};

    #[test]
    fn test_parse_initial_length_32_ok() {
        let section = Section::with_endian(Endian::Little).L32(0x7856_3412);
        let buf = section.get_contents().unwrap();

        let input = &mut EndianSlice::new(&buf, LittleEndian);
        match input.read_initial_length() {
            Ok((length, format)) => {
                assert_eq!(input.len(), 0);
                assert_eq!(format, Format::Dwarf32);
                assert_eq!(0x7856_3412, length);
            }
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        }
    }

    #[test]
    fn test_parse_initial_length_64_ok() {
        let section = Section::with_endian(Endian::Little)
            // Dwarf_64_INITIAL_UNIT_LENGTH
            .L32(0xffff_ffff)
            // Actual length
            .L64(0xffde_bc9a_7856_3412);
        let buf = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&buf, LittleEndian);

        #[cfg(target_pointer_width = "64")]
        match input.read_initial_length() {
            Ok((length, format)) => {
                assert_eq!(input.len(), 0);
                assert_eq!(format, Format::Dwarf64);
                assert_eq!(0xffde_bc9a_7856_3412, length);
            }
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        }

        #[cfg(target_pointer_width = "32")]
        match input.read_initial_length() {
            Err(Error::UnsupportedOffset) => {}
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    fn test_parse_initial_length_unknown_reserved_value() {
        let section = Section::with_endian(Endian::Little).L32(0xffff_fffe);
        let buf = section.get_contents().unwrap();

        let input = &mut EndianSlice::new(&buf, LittleEndian);
        match input.read_initial_length() {
            Err(Error::UnknownReservedLength) => {}
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    fn test_parse_initial_length_incomplete() {
        let buf = [0xff, 0xff, 0xff]; // Need at least 4 bytes.

        let input = &mut EndianSlice::new(&buf, LittleEndian);
        match input.read_initial_length() {
            Err(Error::UnexpectedEof(_)) => {}
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    fn test_parse_initial_length_64_incomplete() {
        let section = Section::with_endian(Endian::Little)
            // Dwarf_64_INITIAL_UNIT_LENGTH
            .L32(0xffff_ffff)
            // Actual length is not long enough.
            .L32(0x7856_3412);
        let buf = section.get_contents().unwrap();

        let input = &mut EndianSlice::new(&buf, LittleEndian);
        match input.read_initial_length() {
            Err(Error::UnexpectedEof(_)) => {}
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    fn test_parse_offset_32() {
        let section = Section::with_endian(Endian::Little).L32(0x0123_4567);
        let buf = section.get_contents().unwrap();

        let input = &mut EndianSlice::new(&buf, LittleEndian);
        match input.read_offset(Format::Dwarf32) {
            Ok(val) => {
                assert_eq!(input.len(), 0);
                assert_eq!(val, 0x0123_4567);
            }
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    fn test_parse_offset_64_small() {
        let section = Section::with_endian(Endian::Little).L64(0x0123_4567);
        let buf = section.get_contents().unwrap();

        let input = &mut EndianSlice::new(&buf, LittleEndian);
        match input.read_offset(Format::Dwarf64) {
            Ok(val) => {
                assert_eq!(input.len(), 0);
                assert_eq!(val, 0x0123_4567);
            }
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_parse_offset_64_large() {
        let section = Section::with_endian(Endian::Little).L64(0x0123_4567_89ab_cdef);
        let buf = section.get_contents().unwrap();

        let input = &mut EndianSlice::new(&buf, LittleEndian);
        match input.read_offset(Format::Dwarf64) {
            Ok(val) => {
                assert_eq!(input.len(), 0);
                assert_eq!(val, 0x0123_4567_89ab_cdef);
            }
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    #[cfg(target_pointer_width = "32")]
    fn test_parse_offset_64_large() {
        let section = Section::with_endian(Endian::Little).L64(0x0123_4567_89ab_cdef);
        let buf = section.get_contents().unwrap();

        let input = &mut EndianSlice::new(&buf, LittleEndian);
        match input.read_offset(Format::Dwarf64) {
            Err(Error::UnsupportedOffset) => {}
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }
}
