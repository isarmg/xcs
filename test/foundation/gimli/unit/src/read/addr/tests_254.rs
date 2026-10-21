#[cfg(test)]
mod tests {
    use super::*;
    use crate::read::EndianSlice;
    use crate::test_util::GimliSectionMethods;
    use crate::{Format, LittleEndian};
    use test_assembler::{Endian, Label, LabelMaker, Section};

    #[test]
    fn test_get_address() {
        for format in [Format::Dwarf32, Format::Dwarf64] {
            for address_size in [4, 8] {
                let zero = Label::new();
                let length = Label::new();
                let start = Label::new();
                let first = Label::new();
                let end = Label::new();
                let mut section = Section::with_endian(Endian::Little)
                    .mark(&zero)
                    .initial_length(format, &length, &start)
                    .D16(5)
                    .D8(address_size)
                    .D8(0)
                    .mark(&first);
                for i in 0..20 {
                    section = section.word(address_size, 1000 + i);
                }
                section = section.mark(&end);
                length.set_const((&end - &start) as u64);

                let section = section.get_contents().unwrap();
                let debug_addr = DebugAddr::from(EndianSlice::new(&section, LittleEndian));
                let base = DebugAddrBase((&first - &zero) as usize);

                assert_eq!(
                    debug_addr.get_address(address_size, base, DebugAddrIndex(0)),
                    Ok(1000)
                );
                assert_eq!(
                    debug_addr.get_address(address_size, base, DebugAddrIndex(19)),
                    Ok(1019)
                );
            }
        }
    }

    #[test]
    fn test_iterator() {
        let length = Label::new();
        let start = Label::new();
        let end = Label::new();
        // First CU.
        let mut section = Section::with_endian(Endian::Little)
            .initial_length(Format::Dwarf32, &length, &start)
            .D16(5) // Version
            .D8(4) // Address size
            .D8(0) // Segment size
            .word(4, 0x12345678)
            .word(4, 0xdeadbeef)
            .mark(&end);
        length.set_const((&end - &start) as u64);
        // Second CU.
        let length = Label::new();
        let start = Label::new();
        let end = Label::new();
        section = section
            .initial_length(Format::Dwarf64, &length, &start)
            .D16(5) // Version
            .D8(8) // Address size
            .D8(0) // Segment size
            .word(8, 0x123456789abcdef0)
            .word(8, 0xdeadbeefdeadbeef)
            .mark(&end);
        length.set_const((&end - &start) as u64);
        let section = section.get_contents().unwrap();
        let debug_addr = DebugAddr::from(EndianSlice::new(&section, LittleEndian));
        let mut iter = debug_addr.headers();
        let first_header = iter.next().unwrap().unwrap();
        let first_encoding = first_header.encoding();
        assert_eq!(first_encoding.address_size, 4);
        assert_eq!(first_encoding.format, Format::Dwarf32);
        assert_eq!(first_encoding.version, 5);
        assert_eq!(first_header.length(), 12);
        let mut first_entries = first_header.entries();
        assert_eq!(first_entries.next(), Ok(Some(0x12345678)));
        assert_eq!(first_entries.next(), Ok(Some(0xdeadbeef)));
        assert_eq!(first_entries.next(), Ok(None));
        let second_header = iter.next().unwrap().unwrap();
        let second_encoding = second_header.encoding();
        assert_eq!(second_encoding.address_size, 8);
        assert_eq!(second_encoding.format, Format::Dwarf64);
        assert_eq!(second_encoding.version, 5);
        assert_eq!(second_header.length(), 20);
        let mut second_entries = second_header.entries();
        assert_eq!(second_entries.next(), Ok(Some(0x123456789abcdef0)));
        assert_eq!(second_entries.next(), Ok(Some(0xdeadbeefdeadbeef)));
        assert_eq!(second_entries.next(), Ok(None));
        assert_eq!(iter.next(), Ok(None));
    }
}
