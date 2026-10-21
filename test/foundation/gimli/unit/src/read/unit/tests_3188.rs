#[cfg(test)]
// Tests require leb128::write.
#[cfg(feature = "write")]
mod tests {
    use super::*;
    use crate::constants;
    use crate::constants::*;
    use crate::endianity::{Endianity, LittleEndian};
    use crate::leb128;
    use crate::read::abbrev::tests::AbbrevSectionMethods;
    use crate::read::{
        Abbreviation, AttributeSpecification, DebugAbbrev, EndianSlice, Error, Result,
    };
    use crate::test_util::GimliSectionMethods;
    use alloc::vec::Vec;
    use core::cell::Cell;
    use test_assembler::{Endian, Label, LabelMaker, Section};

    // Mixin methods for `Section` to help define binary test data.

    trait UnitSectionMethods {
        fn unit<E>(self, unit: &mut UnitHeader<EndianSlice<'_, E>>) -> Self
        where
            E: Endianity;
        fn die<F>(self, code: u64, attr: F) -> Self
        where
            F: Fn(Section) -> Section;
        fn die_null(self) -> Self;
        fn attr_string(self, s: &str) -> Self;
        fn attr_ref1(self, o: u8) -> Self;
        fn offset(self, offset: usize, format: Format) -> Self;
    }

    impl UnitSectionMethods for Section {
        fn unit<E>(self, unit: &mut UnitHeader<EndianSlice<'_, E>>) -> Self
        where
            E: Endianity,
        {
            let size = self.size();
            let length = Label::new();
            let start = Label::new();
            let end = Label::new();

            let section = match unit.format() {
                Format::Dwarf32 => self.L32(&length),
                Format::Dwarf64 => self.L32(0xffff_ffff).L64(&length),
            };

            let section = match unit.version() {
                2..=4 => section
                    .mark(&start)
                    .L16(unit.version())
                    .offset(unit.debug_abbrev_offset.0, unit.format())
                    .D8(unit.address_size()),
                5 => section
                    .mark(&start)
                    .L16(unit.version())
                    .D8(unit.type_().dw_ut().0)
                    .D8(unit.address_size())
                    .offset(unit.debug_abbrev_offset.0, unit.format()),
                _ => unreachable!(),
            };

            let section = match unit.type_() {
                UnitType::Compilation | UnitType::Partial => {
                    unit.unit_offset = DebugInfoOffset(size as usize).into();
                    section
                }
                UnitType::Type {
                    type_signature,
                    type_offset,
                }
                | UnitType::SplitType {
                    type_signature,
                    type_offset,
                } => {
                    if unit.version() == 5 {
                        unit.unit_offset = DebugInfoOffset(size as usize).into();
                    } else {
                        unit.unit_offset = DebugTypesOffset(size as usize).into();
                    }
                    section
                        .L64(type_signature.0)
                        .offset(type_offset.0, unit.format())
                }
                UnitType::Skeleton(dwo_id) | UnitType::SplitCompilation(dwo_id) => {
                    unit.unit_offset = DebugInfoOffset(size as usize).into();
                    section.L64(dwo_id.0)
                }
            };

            let section = section.append_bytes(unit.entries_buf.slice()).mark(&end);

            unit.unit_length = (&end - &start) as usize;
            length.set_const(unit.unit_length as u64);

            section
        }

        fn die<F>(self, code: u64, attr: F) -> Self
        where
            F: Fn(Section) -> Section,
        {
            let section = self.uleb(code);
            attr(section)
        }

        fn die_null(self) -> Self {
            self.D8(0)
        }

        fn attr_string(self, attr: &str) -> Self {
            self.append_bytes(attr.as_bytes()).D8(0)
        }

        fn attr_ref1(self, attr: u8) -> Self {
            self.D8(attr)
        }

        fn offset(self, offset: usize, format: Format) -> Self {
            match format {
                Format::Dwarf32 => self.L32(offset as u32),
                Format::Dwarf64 => self.L64(offset as u64),
            }
        }
    }

    /// Ensure that `UnitHeader<R>` is covariant wrt R.
    #[test]
    fn test_unit_header_variance() {
        /// This only needs to compile.
        fn _f<'a: 'b, 'b, E: Endianity>(
            x: UnitHeader<EndianSlice<'a, E>>,
        ) -> UnitHeader<EndianSlice<'b, E>> {
            x
        }
    }

    #[test]
    fn test_parse_debug_abbrev_offset_32() {
        let section = Section::with_endian(Endian::Little).L32(0x0403_0201);
        let buf = section.get_contents().unwrap();
        let buf = &mut EndianSlice::new(&buf, LittleEndian);

        match parse_debug_abbrev_offset(buf, Format::Dwarf32) {
            Ok(val) => assert_eq!(val, DebugAbbrevOffset(0x0403_0201)),
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    fn test_parse_debug_abbrev_offset_32_incomplete() {
        let buf = [0x01, 0x02];
        let buf = &mut EndianSlice::new(&buf, LittleEndian);

        match parse_debug_abbrev_offset(buf, Format::Dwarf32) {
            Err(Error::UnexpectedEof(_)) => {}
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_parse_debug_abbrev_offset_64() {
        let section = Section::with_endian(Endian::Little).L64(0x0807_0605_0403_0201);
        let buf = section.get_contents().unwrap();
        let buf = &mut EndianSlice::new(&buf, LittleEndian);

        match parse_debug_abbrev_offset(buf, Format::Dwarf64) {
            Ok(val) => assert_eq!(val, DebugAbbrevOffset(0x0807_0605_0403_0201)),
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    fn test_parse_debug_abbrev_offset_64_incomplete() {
        let buf = [0x01, 0x02];
        let buf = &mut EndianSlice::new(&buf, LittleEndian);

        match parse_debug_abbrev_offset(buf, Format::Dwarf64) {
            Err(Error::UnexpectedEof(_)) => {}
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    fn test_parse_debug_info_offset_32() {
        let section = Section::with_endian(Endian::Little).L32(0x0403_0201);
        let buf = section.get_contents().unwrap();
        let buf = &mut EndianSlice::new(&buf, LittleEndian);

        match parse_debug_info_offset(buf, Format::Dwarf32) {
            Ok(val) => assert_eq!(val, DebugInfoOffset(0x0403_0201)),
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    fn test_parse_debug_info_offset_32_incomplete() {
        let buf = [0x01, 0x02];
        let buf = &mut EndianSlice::new(&buf, LittleEndian);

        match parse_debug_info_offset(buf, Format::Dwarf32) {
            Err(Error::UnexpectedEof(_)) => {}
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_parse_debug_info_offset_64() {
        let section = Section::with_endian(Endian::Little).L64(0x0807_0605_0403_0201);
        let buf = section.get_contents().unwrap();
        let buf = &mut EndianSlice::new(&buf, LittleEndian);

        match parse_debug_info_offset(buf, Format::Dwarf64) {
            Ok(val) => assert_eq!(val, DebugInfoOffset(0x0807_0605_0403_0201)),
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    fn test_parse_debug_info_offset_64_incomplete() {
        let buf = [0x01, 0x02];
        let buf = &mut EndianSlice::new(&buf, LittleEndian);

        match parse_debug_info_offset(buf, Format::Dwarf64) {
            Err(Error::UnexpectedEof(_)) => {}
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_units() {
        let expected_rest = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut unit64 = UnitHeader {
            encoding: Encoding {
                format: Format::Dwarf64,
                version: 4,
                address_size: 8,
            },
            unit_length: 0,
            unit_type: UnitType::Compilation,
            debug_abbrev_offset: DebugAbbrevOffset(0x0102_0304_0506_0708),
            unit_offset: DebugInfoOffset(0).into(),
            entries_buf: EndianSlice::new(expected_rest, LittleEndian),
        };
        let mut unit32 = UnitHeader {
            encoding: Encoding {
                format: Format::Dwarf32,
                version: 4,
                address_size: 4,
            },
            unit_length: 0,
            unit_type: UnitType::Compilation,
            debug_abbrev_offset: DebugAbbrevOffset(0x0807_0605),
            unit_offset: DebugInfoOffset(0).into(),
            entries_buf: EndianSlice::new(expected_rest, LittleEndian),
        };
        let section = Section::with_endian(Endian::Little)
            .unit(&mut unit64)
            .unit(&mut unit32);
        let buf = section.get_contents().unwrap();

        let debug_info = DebugInfo::new(&buf, LittleEndian);
        let mut units = debug_info.units();

        assert_eq!(units.next(), Ok(Some(unit64)));
        assert_eq!(units.next(), Ok(Some(unit32)));
        assert_eq!(units.next(), Ok(None));
    }

    #[test]
    fn test_unit_version_unknown_version() {
        let buf = [0x02, 0x00, 0x00, 0x00, 0xab, 0xcd];
        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        match parse_unit_header(rest, DebugInfoOffset(0).into()) {
            Err(Error::UnknownVersion(0xcdab)) => {}
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };

        let buf = [0x02, 0x00, 0x00, 0x00, 0x1, 0x0];
        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        match parse_unit_header(rest, DebugInfoOffset(0).into()) {
            Err(Error::UnknownVersion(1)) => {}
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    fn test_unit_version_incomplete() {
        let buf = [0x01, 0x00, 0x00, 0x00, 0x04];
        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        match parse_unit_header(rest, DebugInfoOffset(0).into()) {
            Err(Error::UnexpectedEof(_)) => {}
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    fn test_parse_unit_header_32_ok() {
        let expected_rest = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 4,
            address_size: 4,
        };
        let mut expected_unit = UnitHeader {
            encoding,
            unit_length: 0,
            unit_type: UnitType::Compilation,
            debug_abbrev_offset: DebugAbbrevOffset(0x0807_0605),
            unit_offset: DebugInfoOffset(0).into(),
            entries_buf: EndianSlice::new(expected_rest, LittleEndian),
        };
        let section = Section::with_endian(Endian::Little)
            .unit(&mut expected_unit)
            .append_bytes(expected_rest);
        let buf = section.get_contents().unwrap();
        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        assert_eq!(
            parse_unit_header(rest, DebugInfoOffset(0).into()),
            Ok(expected_unit)
        );
        assert_eq!(*rest, EndianSlice::new(expected_rest, LittleEndian));
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_parse_unit_header_64_ok() {
        let expected_rest = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let encoding = Encoding {
            format: Format::Dwarf64,
            version: 4,
            address_size: 8,
        };
        let mut expected_unit = UnitHeader {
            encoding,
            unit_length: 0,
            unit_type: UnitType::Compilation,
            debug_abbrev_offset: DebugAbbrevOffset(0x0102_0304_0506_0708),
            unit_offset: DebugInfoOffset(0).into(),
            entries_buf: EndianSlice::new(expected_rest, LittleEndian),
        };
        let section = Section::with_endian(Endian::Little)
            .unit(&mut expected_unit)
            .append_bytes(expected_rest);
        let buf = section.get_contents().unwrap();
        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        assert_eq!(
            parse_unit_header(rest, DebugInfoOffset(0).into()),
            Ok(expected_unit)
        );
        assert_eq!(*rest, EndianSlice::new(expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_v5_unit_header_32_ok() {
        let expected_rest = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 5,
            address_size: 4,
        };
        let mut expected_unit = UnitHeader {
            encoding,
            unit_length: 0,
            unit_type: UnitType::Compilation,
            debug_abbrev_offset: DebugAbbrevOffset(0x0807_0605),
            unit_offset: DebugInfoOffset(0).into(),
            entries_buf: EndianSlice::new(expected_rest, LittleEndian),
        };
        let section = Section::with_endian(Endian::Little)
            .unit(&mut expected_unit)
            .append_bytes(expected_rest);
        let buf = section.get_contents().unwrap();
        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        assert_eq!(
            parse_unit_header(rest, DebugInfoOffset(0).into()),
            Ok(expected_unit)
        );
        assert_eq!(*rest, EndianSlice::new(expected_rest, LittleEndian));
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_parse_v5_unit_header_64_ok() {
        let expected_rest = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let encoding = Encoding {
            format: Format::Dwarf64,
            version: 5,
            address_size: 8,
        };
        let mut expected_unit = UnitHeader {
            encoding,
            unit_length: 0,
            unit_type: UnitType::Compilation,
            debug_abbrev_offset: DebugAbbrevOffset(0x0102_0304_0506_0708),
            unit_offset: DebugInfoOffset(0).into(),
            entries_buf: EndianSlice::new(expected_rest, LittleEndian),
        };
        let section = Section::with_endian(Endian::Little)
            .unit(&mut expected_unit)
            .append_bytes(expected_rest);
        let buf = section.get_contents().unwrap();
        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        assert_eq!(
            parse_unit_header(rest, DebugInfoOffset(0).into()),
            Ok(expected_unit)
        );
        assert_eq!(*rest, EndianSlice::new(expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_v5_partial_unit_header_32_ok() {
        let expected_rest = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 5,
            address_size: 4,
        };
        let mut expected_unit = UnitHeader {
            encoding,
            unit_length: 0,
            unit_type: UnitType::Partial,
            debug_abbrev_offset: DebugAbbrevOffset(0x0807_0605),
            unit_offset: DebugInfoOffset(0).into(),
            entries_buf: EndianSlice::new(expected_rest, LittleEndian),
        };
        let section = Section::with_endian(Endian::Little)
            .unit(&mut expected_unit)
            .append_bytes(expected_rest);
        let buf = section.get_contents().unwrap();
        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        assert_eq!(
            parse_unit_header(rest, DebugInfoOffset(0).into()),
            Ok(expected_unit)
        );
        assert_eq!(*rest, EndianSlice::new(expected_rest, LittleEndian));
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_parse_v5_partial_unit_header_64_ok() {
        let expected_rest = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let encoding = Encoding {
            format: Format::Dwarf64,
            version: 5,
            address_size: 8,
        };
        let mut expected_unit = UnitHeader {
            encoding,
            unit_length: 0,
            unit_type: UnitType::Partial,
            debug_abbrev_offset: DebugAbbrevOffset(0x0102_0304_0506_0708),
            unit_offset: DebugInfoOffset(0).into(),
            entries_buf: EndianSlice::new(expected_rest, LittleEndian),
        };
        let section = Section::with_endian(Endian::Little)
            .unit(&mut expected_unit)
            .append_bytes(expected_rest);
        let buf = section.get_contents().unwrap();
        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        assert_eq!(
            parse_unit_header(rest, DebugInfoOffset(0).into()),
            Ok(expected_unit)
        );
        assert_eq!(*rest, EndianSlice::new(expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_v5_skeleton_unit_header_32_ok() {
        let expected_rest = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 5,
            address_size: 4,
        };
        let mut expected_unit = UnitHeader {
            encoding,
            unit_length: 0,
            unit_type: UnitType::Skeleton(DwoId(0x0706_5040_0302_1000)),
            debug_abbrev_offset: DebugAbbrevOffset(0x0807_0605),
            unit_offset: DebugInfoOffset(0).into(),
            entries_buf: EndianSlice::new(expected_rest, LittleEndian),
        };
        let section = Section::with_endian(Endian::Little)
            .unit(&mut expected_unit)
            .append_bytes(expected_rest);
        let buf = section.get_contents().unwrap();
        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        assert_eq!(
            parse_unit_header(rest, DebugInfoOffset(0).into()),
            Ok(expected_unit)
        );
        assert_eq!(*rest, EndianSlice::new(expected_rest, LittleEndian));
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_parse_v5_skeleton_unit_header_64_ok() {
        let expected_rest = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let encoding = Encoding {
            format: Format::Dwarf64,
            version: 5,
            address_size: 8,
        };
        let mut expected_unit = UnitHeader {
            encoding,
            unit_length: 0,
            unit_type: UnitType::Skeleton(DwoId(0x0706_5040_0302_1000)),
            debug_abbrev_offset: DebugAbbrevOffset(0x0102_0304_0506_0708),
            unit_offset: DebugInfoOffset(0).into(),
            entries_buf: EndianSlice::new(expected_rest, LittleEndian),
        };
        let section = Section::with_endian(Endian::Little)
            .unit(&mut expected_unit)
            .append_bytes(expected_rest);
        let buf = section.get_contents().unwrap();
        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        assert_eq!(
            parse_unit_header(rest, DebugInfoOffset(0).into()),
            Ok(expected_unit)
        );
        assert_eq!(*rest, EndianSlice::new(expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_v5_split_compilation_unit_header_32_ok() {
        let expected_rest = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 5,
            address_size: 4,
        };
        let mut expected_unit = UnitHeader {
            encoding,
            unit_length: 0,
            unit_type: UnitType::SplitCompilation(DwoId(0x0706_5040_0302_1000)),
            debug_abbrev_offset: DebugAbbrevOffset(0x0807_0605),
            unit_offset: DebugInfoOffset(0).into(),
            entries_buf: EndianSlice::new(expected_rest, LittleEndian),
        };
        let section = Section::with_endian(Endian::Little)
            .unit(&mut expected_unit)
            .append_bytes(expected_rest);
        let buf = section.get_contents().unwrap();
        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        assert_eq!(
            parse_unit_header(rest, DebugInfoOffset(0).into()),
            Ok(expected_unit)
        );
        assert_eq!(*rest, EndianSlice::new(expected_rest, LittleEndian));
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_parse_v5_split_compilation_unit_header_64_ok() {
        let expected_rest = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let encoding = Encoding {
            format: Format::Dwarf64,
            version: 5,
            address_size: 8,
        };
        let mut expected_unit = UnitHeader {
            encoding,
            unit_length: 0,
            unit_type: UnitType::SplitCompilation(DwoId(0x0706_5040_0302_1000)),
            debug_abbrev_offset: DebugAbbrevOffset(0x0102_0304_0506_0708),
            unit_offset: DebugInfoOffset(0).into(),
            entries_buf: EndianSlice::new(expected_rest, LittleEndian),
        };
        let section = Section::with_endian(Endian::Little)
            .unit(&mut expected_unit)
            .append_bytes(expected_rest);
        let buf = section.get_contents().unwrap();
        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        assert_eq!(
            parse_unit_header(rest, DebugInfoOffset(0).into()),
            Ok(expected_unit)
        );
        assert_eq!(*rest, EndianSlice::new(expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_type_offset_32_ok() {
        let buf = [0x12, 0x34, 0x56, 0x78, 0x00];
        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        match parse_type_offset(rest, Format::Dwarf32) {
            Ok(offset) => {
                assert_eq!(rest.len(), 1);
                assert_eq!(UnitOffset(0x7856_3412), offset);
            }
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        }
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_parse_type_offset_64_ok() {
        let buf = [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xff, 0x00];
        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        match parse_type_offset(rest, Format::Dwarf64) {
            Ok(offset) => {
                assert_eq!(rest.len(), 1);
                assert_eq!(UnitOffset(0xffde_bc9a_7856_3412), offset);
            }
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        }
    }

    #[test]
    fn test_parse_type_offset_incomplete() {
        // Need at least 4 bytes.
        let buf = [0xff, 0xff, 0xff];
        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        match parse_type_offset(rest, Format::Dwarf32) {
            Err(Error::UnexpectedEof(_)) => {}
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    fn test_parse_type_unit_header_32_ok() {
        let expected_rest = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 4,
            address_size: 8,
        };
        let mut expected_unit = UnitHeader {
            encoding,
            unit_length: 0,
            unit_type: UnitType::Type {
                type_signature: DebugTypeSignature(0xdead_beef_dead_beef),
                type_offset: UnitOffset(0x7856_3412),
            },
            debug_abbrev_offset: DebugAbbrevOffset(0x0807_0605),
            unit_offset: DebugTypesOffset(0).into(),
            entries_buf: EndianSlice::new(expected_rest, LittleEndian),
        };
        let section = Section::with_endian(Endian::Little)
            .unit(&mut expected_unit)
            .append_bytes(expected_rest);
        let buf = section.get_contents().unwrap();
        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        assert_eq!(
            parse_unit_header(rest, DebugTypesOffset(0).into()),
            Ok(expected_unit)
        );
        assert_eq!(*rest, EndianSlice::new(expected_rest, LittleEndian));
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_parse_type_unit_header_64_ok() {
        let expected_rest = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let encoding = Encoding {
            format: Format::Dwarf64,
            version: 4,
            address_size: 8,
        };
        let mut expected_unit = UnitHeader {
            encoding,
            unit_length: 0,
            unit_type: UnitType::Type {
                type_signature: DebugTypeSignature(0xdead_beef_dead_beef),
                type_offset: UnitOffset(0x7856_3412_7856_3412),
            },
            debug_abbrev_offset: DebugAbbrevOffset(0x0807_0605),
            unit_offset: DebugTypesOffset(0).into(),
            entries_buf: EndianSlice::new(expected_rest, LittleEndian),
        };
        let section = Section::with_endian(Endian::Little)
            .unit(&mut expected_unit)
            .append_bytes(expected_rest);
        let buf = section.get_contents().unwrap();
        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        assert_eq!(
            parse_unit_header(rest, DebugTypesOffset(0).into()),
            Ok(expected_unit)
        );
        assert_eq!(*rest, EndianSlice::new(expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_v5_type_unit_header_32_ok() {
        let expected_rest = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 5,
            address_size: 8,
        };
        let mut expected_unit = UnitHeader {
            encoding,
            unit_length: 0,
            unit_type: UnitType::Type {
                type_signature: DebugTypeSignature(0xdead_beef_dead_beef),
                type_offset: UnitOffset(0x7856_3412),
            },
            debug_abbrev_offset: DebugAbbrevOffset(0x0807_0605),
            unit_offset: DebugInfoOffset(0).into(),
            entries_buf: EndianSlice::new(expected_rest, LittleEndian),
        };
        let section = Section::with_endian(Endian::Little)
            .unit(&mut expected_unit)
            .append_bytes(expected_rest);
        let buf = section.get_contents().unwrap();
        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        assert_eq!(
            parse_unit_header(rest, DebugInfoOffset(0).into()),
            Ok(expected_unit)
        );
        assert_eq!(*rest, EndianSlice::new(expected_rest, LittleEndian));
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_parse_v5_type_unit_header_64_ok() {
        let expected_rest = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let encoding = Encoding {
            format: Format::Dwarf64,
            version: 5,
            address_size: 8,
        };
        let mut expected_unit = UnitHeader {
            encoding,
            unit_length: 0,
            unit_type: UnitType::Type {
                type_signature: DebugTypeSignature(0xdead_beef_dead_beef),
                type_offset: UnitOffset(0x7856_3412_7856_3412),
            },
            debug_abbrev_offset: DebugAbbrevOffset(0x0807_0605),
            unit_offset: DebugInfoOffset(0).into(),
            entries_buf: EndianSlice::new(expected_rest, LittleEndian),
        };
        let section = Section::with_endian(Endian::Little)
            .unit(&mut expected_unit)
            .append_bytes(expected_rest);
        let buf = section.get_contents().unwrap();
        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        assert_eq!(
            parse_unit_header(rest, DebugInfoOffset(0).into()),
            Ok(expected_unit)
        );
        assert_eq!(*rest, EndianSlice::new(expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_v5_split_type_unit_header_32_ok() {
        let expected_rest = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 5,
            address_size: 8,
        };
        let mut expected_unit = UnitHeader {
            encoding,
            unit_length: 0,
            unit_type: UnitType::SplitType {
                type_signature: DebugTypeSignature(0xdead_beef_dead_beef),
                type_offset: UnitOffset(0x7856_3412),
            },
            debug_abbrev_offset: DebugAbbrevOffset(0x0807_0605),
            unit_offset: DebugInfoOffset(0).into(),
            entries_buf: EndianSlice::new(expected_rest, LittleEndian),
        };
        let section = Section::with_endian(Endian::Little)
            .unit(&mut expected_unit)
            .append_bytes(expected_rest);
        let buf = section.get_contents().unwrap();
        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        assert_eq!(
            parse_unit_header(rest, DebugInfoOffset(0).into()),
            Ok(expected_unit)
        );
        assert_eq!(*rest, EndianSlice::new(expected_rest, LittleEndian));
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_parse_v5_split_type_unit_header_64_ok() {
        let expected_rest = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
        let encoding = Encoding {
            format: Format::Dwarf64,
            version: 5,
            address_size: 8,
        };
        let mut expected_unit = UnitHeader {
            encoding,
            unit_length: 0,
            unit_type: UnitType::SplitType {
                type_signature: DebugTypeSignature(0xdead_beef_dead_beef),
                type_offset: UnitOffset(0x7856_3412_7856_3412),
            },
            debug_abbrev_offset: DebugAbbrevOffset(0x0807_0605),
            unit_offset: DebugInfoOffset(0).into(),
            entries_buf: EndianSlice::new(expected_rest, LittleEndian),
        };
        let section = Section::with_endian(Endian::Little)
            .unit(&mut expected_unit)
            .append_bytes(expected_rest);
        let buf = section.get_contents().unwrap();
        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        assert_eq!(
            parse_unit_header(rest, DebugInfoOffset(0).into()),
            Ok(expected_unit)
        );
        assert_eq!(*rest, EndianSlice::new(expected_rest, LittleEndian));
    }

    fn section_contents<F>(f: F) -> Vec<u8>
    where
        F: Fn(Section) -> Section,
    {
        f(Section::with_endian(Endian::Little))
            .get_contents()
            .unwrap()
    }

    #[test]
    fn test_attribute_value() {
        let mut unit = test_parse_attribute_unit_default();
        let endian = unit.entries_buf.endian();

        let block_data = &[1, 2, 3, 4];
        let buf = section_contents(|s| s.uleb(block_data.len() as u64).append_bytes(block_data));
        let block = EndianSlice::new(&buf, endian);

        let buf = section_contents(|s| s.L32(0x0102_0304));
        let data4 = EndianSlice::new(&buf, endian);

        let buf = section_contents(|s| s.L64(0x0102_0304_0506_0708));
        let data8 = EndianSlice::new(&buf, endian);

        let tests = [
            (
                Format::Dwarf32,
                2,
                constants::DW_AT_data_member_location,
                constants::DW_FORM_block,
                block,
                AttributeValue::Block(EndianSlice::new(block_data, endian)),
                AttributeValue::Exprloc(Expression(EndianSlice::new(block_data, endian))),
            ),
            (
                Format::Dwarf32,
                2,
                constants::DW_AT_data_member_location,
                constants::DW_FORM_data4,
                data4,
                AttributeValue::SecOffset(0x0102_0304),
                AttributeValue::LocationListsRef(LocationListsOffset(0x0102_0304)),
            ),
            (
                Format::Dwarf64,
                2,
                constants::DW_AT_data_member_location,
                constants::DW_FORM_data4,
                data4,
                AttributeValue::Data4(0x0102_0304),
                AttributeValue::Udata(0x0102_0304),
            ),
            (
                Format::Dwarf32,
                4,
                constants::DW_AT_data_member_location,
                constants::DW_FORM_data4,
                data4,
                AttributeValue::Data4(0x0102_0304),
                AttributeValue::Udata(0x0102_0304),
            ),
            (
                Format::Dwarf32,
                2,
                constants::DW_AT_data_member_location,
                constants::DW_FORM_data8,
                data8,
                AttributeValue::Data8(0x0102_0304_0506_0708),
                AttributeValue::Udata(0x0102_0304_0506_0708),
            ),
            #[cfg(target_pointer_width = "64")]
            (
                Format::Dwarf64,
                2,
                constants::DW_AT_data_member_location,
                constants::DW_FORM_data8,
                data8,
                AttributeValue::SecOffset(0x0102_0304_0506_0708),
                AttributeValue::LocationListsRef(LocationListsOffset(0x0102_0304_0506_0708)),
            ),
            (
                Format::Dwarf64,
                4,
                constants::DW_AT_data_member_location,
                constants::DW_FORM_data8,
                data8,
                AttributeValue::Data8(0x0102_0304_0506_0708),
                AttributeValue::Udata(0x0102_0304_0506_0708),
            ),
            (
                Format::Dwarf32,
                4,
                constants::DW_AT_location,
                constants::DW_FORM_data4,
                data4,
                AttributeValue::SecOffset(0x0102_0304),
                AttributeValue::LocationListsRef(LocationListsOffset(0x0102_0304)),
            ),
            #[cfg(target_pointer_width = "64")]
            (
                Format::Dwarf64,
                4,
                constants::DW_AT_location,
                constants::DW_FORM_data8,
                data8,
                AttributeValue::SecOffset(0x0102_0304_0506_0708),
                AttributeValue::LocationListsRef(LocationListsOffset(0x0102_0304_0506_0708)),
            ),
            (
                Format::Dwarf32,
                4,
                constants::DW_AT_str_offsets_base,
                constants::DW_FORM_sec_offset,
                data4,
                AttributeValue::SecOffset(0x0102_0304),
                AttributeValue::DebugStrOffsetsBase(DebugStrOffsetsBase(0x0102_0304)),
            ),
            (
                Format::Dwarf32,
                4,
                constants::DW_AT_stmt_list,
                constants::DW_FORM_sec_offset,
                data4,
                AttributeValue::SecOffset(0x0102_0304),
                AttributeValue::DebugLineRef(DebugLineOffset(0x0102_0304)),
            ),
            (
                Format::Dwarf32,
                4,
                constants::DW_AT_addr_base,
                constants::DW_FORM_sec_offset,
                data4,
                AttributeValue::SecOffset(0x0102_0304),
                AttributeValue::DebugAddrBase(DebugAddrBase(0x0102_0304)),
            ),
            (
                Format::Dwarf32,
                4,
                constants::DW_AT_rnglists_base,
                constants::DW_FORM_sec_offset,
                data4,
                AttributeValue::SecOffset(0x0102_0304),
                AttributeValue::DebugRngListsBase(DebugRngListsBase(0x0102_0304)),
            ),
            (
                Format::Dwarf32,
                4,
                constants::DW_AT_loclists_base,
                constants::DW_FORM_sec_offset,
                data4,
                AttributeValue::SecOffset(0x0102_0304),
                AttributeValue::DebugLocListsBase(DebugLocListsBase(0x0102_0304)),
            ),
        ];

        for test in tests.iter() {
            let (format, version, name, form, mut input, expect_raw, expect_value) = *test;
            unit.encoding.format = format;
            unit.encoding.version = version;
            let spec = AttributeSpecification::new(name, form, None);
            let attribute =
                parse_attribute(&mut input, unit.encoding(), spec).expect("Should parse attribute");
            assert_eq!(attribute.raw_value(), expect_raw);
            assert_eq!(attribute.value(), expect_value);
        }
    }

    #[test]
    fn test_attribute_udata_sdata_value() {
        #[allow(clippy::type_complexity)]
        let tests: &[(
            AttributeValue<EndianSlice<'_, LittleEndian>>,
            Option<u64>,
            Option<i64>,
        )] = &[
            (AttributeValue::Data1(1), Some(1), Some(1)),
            (
                AttributeValue::Data1(u8::MAX),
                Some(u64::from(u8::MAX)),
                Some(-1),
            ),
            (AttributeValue::Data2(1), Some(1), Some(1)),
            (
                AttributeValue::Data2(u16::MAX),
                Some(u64::from(u16::MAX)),
                Some(-1),
            ),
            (AttributeValue::Data4(1), Some(1), Some(1)),
            (
                AttributeValue::Data4(u32::MAX),
                Some(u64::from(u32::MAX)),
                Some(-1),
            ),
            (AttributeValue::Data8(1), Some(1), Some(1)),
            (AttributeValue::Data8(u64::MAX), Some(u64::MAX), Some(-1)),
            (AttributeValue::Sdata(1), Some(1), Some(1)),
            (AttributeValue::Sdata(-1), None, Some(-1)),
            (AttributeValue::Udata(1), Some(1), Some(1)),
            (AttributeValue::Udata(1u64 << 63), Some(1u64 << 63), None),
        ];
        for test in tests.iter() {
            let (value, expect_udata, expect_sdata) = *test;
            let attribute = Attribute {
                name: DW_AT_data_member_location,
                value,
            };
            assert_eq!(attribute.udata_value(), expect_udata);
            assert_eq!(attribute.sdata_value(), expect_sdata);
        }
    }

    fn test_parse_attribute_unit<Endian>(
        address_size: u8,
        format: Format,
        endian: Endian,
    ) -> UnitHeader<EndianSlice<'static, Endian>>
    where
        Endian: Endianity,
    {
        let encoding = Encoding {
            format,
            version: 4,
            address_size,
        };
        UnitHeader::new(
            encoding,
            7,
            UnitType::Compilation,
            DebugAbbrevOffset(0x0807_0605),
            DebugInfoOffset(0).into(),
            EndianSlice::new(&[], endian),
        )
    }

    fn test_parse_attribute_unit_default() -> UnitHeader<EndianSlice<'static, LittleEndian>> {
        test_parse_attribute_unit(4, Format::Dwarf32, LittleEndian)
    }

    fn test_parse_attribute<'input, Endian>(
        buf: &'input [u8],
        len: usize,
        unit: &UnitHeader<EndianSlice<'input, Endian>>,
        form: constants::DwForm,
        value: AttributeValue<EndianSlice<'input, Endian>>,
    ) where
        Endian: Endianity,
    {
        let spec = AttributeSpecification::new(constants::DW_AT_low_pc, form, None);

        let expect = Attribute {
            name: constants::DW_AT_low_pc,
            value,
        };

        let rest = &mut EndianSlice::new(buf, Endian::default());
        match parse_attribute(rest, unit.encoding(), spec) {
            Ok(attr) => {
                assert_eq!(attr, expect);
                assert_eq!(*rest, EndianSlice::new(&buf[len..], Endian::default()));
                if let Some(size) = spec.size(unit) {
                    assert_eq!(rest.len() + size, buf.len());
                }
            }
            otherwise => {
                panic!("Unexpected parse result = {:#?}", otherwise);
            }
        };
    }

    #[test]
    fn test_parse_attribute_addr() {
        let buf = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let unit = test_parse_attribute_unit(4, Format::Dwarf32, LittleEndian);
        let form = constants::DW_FORM_addr;
        let value = AttributeValue::Addr(0x0403_0201);
        test_parse_attribute(&buf, 4, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_addr8() {
        let buf = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let unit = test_parse_attribute_unit(8, Format::Dwarf32, LittleEndian);
        let form = constants::DW_FORM_addr;
        let value = AttributeValue::Addr(0x0807_0605_0403_0201);
        test_parse_attribute(&buf, 8, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_block1() {
        // Length of data (3), three bytes of data, two bytes of left over input.
        let buf = [0x03, 0x09, 0x09, 0x09, 0x00, 0x00];
        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_block1;
        let value = AttributeValue::Block(EndianSlice::new(&buf[1..4], LittleEndian));
        test_parse_attribute(&buf, 4, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_block2() {
        // Two byte length of data (2), two bytes of data, two bytes of left over input.
        let buf = [0x02, 0x00, 0x09, 0x09, 0x00, 0x00];
        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_block2;
        let value = AttributeValue::Block(EndianSlice::new(&buf[2..4], LittleEndian));
        test_parse_attribute(&buf, 4, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_block4() {
        // Four byte length of data (2), two bytes of data, no left over input.
        let buf = [0x02, 0x00, 0x00, 0x00, 0x99, 0x99];
        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_block4;
        let value = AttributeValue::Block(EndianSlice::new(&buf[4..], LittleEndian));
        test_parse_attribute(&buf, 6, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_block() {
        // LEB length of data (2, one byte), two bytes of data, no left over input.
        let buf = [0x02, 0x99, 0x99];
        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_block;
        let value = AttributeValue::Block(EndianSlice::new(&buf[1..], LittleEndian));
        test_parse_attribute(&buf, 3, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_data1() {
        let buf = [0x03];
        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_data1;
        let value = AttributeValue::Data1(0x03);
        test_parse_attribute(&buf, 1, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_data2() {
        let buf = [0x02, 0x01, 0x0];
        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_data2;
        let value = AttributeValue::Data2(0x0102);
        test_parse_attribute(&buf, 2, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_data4() {
        let buf = [0x01, 0x02, 0x03, 0x04, 0x99, 0x99];
        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_data4;
        let value = AttributeValue::Data4(0x0403_0201);
        test_parse_attribute(&buf, 4, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_data8() {
        let buf = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x99, 0x99];
        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_data8;
        let value = AttributeValue::Data8(0x0807_0605_0403_0201);
        test_parse_attribute(&buf, 8, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_udata() {
        let mut buf = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

        let bytes_written = {
            let mut writable = &mut buf[..];
            leb128::write::unsigned(&mut writable, 4097).expect("should write ok")
        };

        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_udata;
        let value = AttributeValue::Udata(4097);
        test_parse_attribute(&buf, bytes_written, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_sdata() {
        let mut buf = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

        let bytes_written = {
            let mut writable = &mut buf[..];
            leb128::write::signed(&mut writable, -4097).expect("should write ok")
        };

        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_sdata;
        let value = AttributeValue::Sdata(-4097);
        test_parse_attribute(&buf, bytes_written, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_exprloc() {
        // LEB length of data (2, one byte), two bytes of data, one byte left over input.
        let buf = [0x02, 0x99, 0x99, 0x11];
        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_exprloc;
        let value = AttributeValue::Exprloc(Expression(EndianSlice::new(&buf[1..3], LittleEndian)));
        test_parse_attribute(&buf, 3, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_flag_true() {
        let buf = [0x42];
        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_flag;
        let value = AttributeValue::Flag(true);
        test_parse_attribute(&buf, 1, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_flag_false() {
        let buf = [0x00];
        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_flag;
        let value = AttributeValue::Flag(false);
        test_parse_attribute(&buf, 1, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_flag_present() {
        let buf = [0x01, 0x02, 0x03, 0x04];
        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_flag_present;
        let value = AttributeValue::Flag(true);
        // DW_FORM_flag_present does not consume any bytes of the input stream.
        test_parse_attribute(&buf, 0, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_sec_offset_32() {
        let buf = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x10];
        let unit = test_parse_attribute_unit(4, Format::Dwarf32, LittleEndian);
        let form = constants::DW_FORM_sec_offset;
        let value = AttributeValue::SecOffset(0x0403_0201);
        test_parse_attribute(&buf, 4, &unit, form, value);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_parse_attribute_sec_offset_64() {
        let buf = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x10];
        let unit = test_parse_attribute_unit(4, Format::Dwarf64, LittleEndian);
        let form = constants::DW_FORM_sec_offset;
        let value = AttributeValue::SecOffset(0x0807_0605_0403_0201);
        test_parse_attribute(&buf, 8, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_ref1() {
        let buf = [0x03];
        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_ref1;
        let value = AttributeValue::UnitRef(UnitOffset(3));
        test_parse_attribute(&buf, 1, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_ref2() {
        let buf = [0x02, 0x01, 0x0];
        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_ref2;
        let value = AttributeValue::UnitRef(UnitOffset(258));
        test_parse_attribute(&buf, 2, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_ref4() {
        let buf = [0x01, 0x02, 0x03, 0x04, 0x99, 0x99];
        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_ref4;
        let value = AttributeValue::UnitRef(UnitOffset(0x0403_0201));
        test_parse_attribute(&buf, 4, &unit, form, value);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_parse_attribute_ref8() {
        let buf = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x99, 0x99];
        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_ref8;
        let value = AttributeValue::UnitRef(UnitOffset(0x0807_0605_0403_0201));
        test_parse_attribute(&buf, 8, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_ref_sup4() {
        let buf = [0x01, 0x02, 0x03, 0x04, 0x99, 0x99];
        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_ref_sup4;
        let value = AttributeValue::DebugInfoRefSup(DebugInfoOffset(0x0403_0201));
        test_parse_attribute(&buf, 4, &unit, form, value);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_parse_attribute_ref_sup8() {
        let buf = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x99, 0x99];
        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_ref_sup8;
        let value = AttributeValue::DebugInfoRefSup(DebugInfoOffset(0x0807_0605_0403_0201));
        test_parse_attribute(&buf, 8, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_refudata() {
        let mut buf = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

        let bytes_written = {
            let mut writable = &mut buf[..];
            leb128::write::unsigned(&mut writable, 4097).expect("should write ok")
        };

        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_ref_udata;
        let value = AttributeValue::UnitRef(UnitOffset(4097));
        test_parse_attribute(&buf, bytes_written, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_refaddr_32() {
        let buf = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x99, 0x99];
        let unit = test_parse_attribute_unit(4, Format::Dwarf32, LittleEndian);
        let form = constants::DW_FORM_ref_addr;
        let value = AttributeValue::DebugInfoRef(DebugInfoOffset(0x0403_0201));
        test_parse_attribute(&buf, 4, &unit, form, value);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_parse_attribute_refaddr_64() {
        let buf = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x99, 0x99];
        let unit = test_parse_attribute_unit(4, Format::Dwarf64, LittleEndian);
        let form = constants::DW_FORM_ref_addr;
        let value = AttributeValue::DebugInfoRef(DebugInfoOffset(0x0807_0605_0403_0201));
        test_parse_attribute(&buf, 8, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_refaddr_version2() {
        let buf = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x99, 0x99];
        let mut unit = test_parse_attribute_unit(4, Format::Dwarf32, LittleEndian);
        unit.encoding.version = 2;
        let form = constants::DW_FORM_ref_addr;
        let value = AttributeValue::DebugInfoRef(DebugInfoOffset(0x0403_0201));
        test_parse_attribute(&buf, 4, &unit, form, value);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_parse_attribute_refaddr8_version2() {
        let buf = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x99, 0x99];
        let mut unit = test_parse_attribute_unit(8, Format::Dwarf32, LittleEndian);
        unit.encoding.version = 2;
        let form = constants::DW_FORM_ref_addr;
        let value = AttributeValue::DebugInfoRef(DebugInfoOffset(0x0807_0605_0403_0201));
        test_parse_attribute(&buf, 8, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_gnu_ref_alt_32() {
        let buf = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x99, 0x99];
        let unit = test_parse_attribute_unit(4, Format::Dwarf32, LittleEndian);
        let form = constants::DW_FORM_GNU_ref_alt;
        let value = AttributeValue::DebugInfoRefSup(DebugInfoOffset(0x0403_0201));
        test_parse_attribute(&buf, 4, &unit, form, value);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_parse_attribute_gnu_ref_alt_64() {
        let buf = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x99, 0x99];
        let unit = test_parse_attribute_unit(4, Format::Dwarf64, LittleEndian);
        let form = constants::DW_FORM_GNU_ref_alt;
        let value = AttributeValue::DebugInfoRefSup(DebugInfoOffset(0x0807_0605_0403_0201));
        test_parse_attribute(&buf, 8, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_refsig8() {
        let buf = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x99, 0x99];
        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_ref_sig8;
        let value = AttributeValue::DebugTypesRef(DebugTypeSignature(0x0807_0605_0403_0201));
        test_parse_attribute(&buf, 8, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_string() {
        let buf = [0x01, 0x02, 0x03, 0x04, 0x05, 0x0, 0x99, 0x99];
        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_string;
        let value = AttributeValue::String(EndianSlice::new(&buf[..5], LittleEndian));
        test_parse_attribute(&buf, 6, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_strp_32() {
        let buf = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x99, 0x99];
        let unit = test_parse_attribute_unit(4, Format::Dwarf32, LittleEndian);
        let form = constants::DW_FORM_strp;
        let value = AttributeValue::DebugStrRef(DebugStrOffset(0x0403_0201));
        test_parse_attribute(&buf, 4, &unit, form, value);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_parse_attribute_strp_64() {
        let buf = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x99, 0x99];
        let unit = test_parse_attribute_unit(4, Format::Dwarf64, LittleEndian);
        let form = constants::DW_FORM_strp;
        let value = AttributeValue::DebugStrRef(DebugStrOffset(0x0807_0605_0403_0201));
        test_parse_attribute(&buf, 8, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_strp_sup_32() {
        let buf = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x99, 0x99];
        let unit = test_parse_attribute_unit(4, Format::Dwarf32, LittleEndian);
        let form = constants::DW_FORM_strp_sup;
        let value = AttributeValue::DebugStrRefSup(DebugStrOffset(0x0403_0201));
        test_parse_attribute(&buf, 4, &unit, form, value);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_parse_attribute_strp_sup_64() {
        let buf = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x99, 0x99];
        let unit = test_parse_attribute_unit(4, Format::Dwarf64, LittleEndian);
        let form = constants::DW_FORM_strp_sup;
        let value = AttributeValue::DebugStrRefSup(DebugStrOffset(0x0807_0605_0403_0201));
        test_parse_attribute(&buf, 8, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_gnu_strp_alt_32() {
        let buf = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x99, 0x99];
        let unit = test_parse_attribute_unit(4, Format::Dwarf32, LittleEndian);
        let form = constants::DW_FORM_GNU_strp_alt;
        let value = AttributeValue::DebugStrRefSup(DebugStrOffset(0x0403_0201));
        test_parse_attribute(&buf, 4, &unit, form, value);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn test_parse_attribute_gnu_strp_alt_64() {
        let buf = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x99, 0x99];
        let unit = test_parse_attribute_unit(4, Format::Dwarf64, LittleEndian);
        let form = constants::DW_FORM_GNU_strp_alt;
        let value = AttributeValue::DebugStrRefSup(DebugStrOffset(0x0807_0605_0403_0201));
        test_parse_attribute(&buf, 8, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_strx() {
        let mut buf = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

        let bytes_written = {
            let mut writable = &mut buf[..];
            leb128::write::unsigned(&mut writable, 4097).expect("should write ok")
        };

        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_strx;
        let value = AttributeValue::DebugStrOffsetsIndex(DebugStrOffsetsIndex(4097));
        test_parse_attribute(&buf, bytes_written, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_strx1() {
        let buf = [0x01, 0x99, 0x99];
        let unit = test_parse_attribute_unit(4, Format::Dwarf64, LittleEndian);
        let form = constants::DW_FORM_strx1;
        let value = AttributeValue::DebugStrOffsetsIndex(DebugStrOffsetsIndex(0x01));
        test_parse_attribute(&buf, 1, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_strx2() {
        let buf = [0x01, 0x02, 0x99, 0x99];
        let unit = test_parse_attribute_unit(4, Format::Dwarf64, LittleEndian);
        let form = constants::DW_FORM_strx2;
        let value = AttributeValue::DebugStrOffsetsIndex(DebugStrOffsetsIndex(0x0201));
        test_parse_attribute(&buf, 2, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_strx3() {
        let buf = [0x01, 0x02, 0x03, 0x99, 0x99];
        let unit = test_parse_attribute_unit(4, Format::Dwarf64, LittleEndian);
        let form = constants::DW_FORM_strx3;
        let value = AttributeValue::DebugStrOffsetsIndex(DebugStrOffsetsIndex(0x03_0201));
        test_parse_attribute(&buf, 3, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_strx4() {
        let buf = [0x01, 0x02, 0x03, 0x04, 0x99, 0x99];
        let unit = test_parse_attribute_unit(4, Format::Dwarf64, LittleEndian);
        let form = constants::DW_FORM_strx4;
        let value = AttributeValue::DebugStrOffsetsIndex(DebugStrOffsetsIndex(0x0403_0201));
        test_parse_attribute(&buf, 4, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_addrx() {
        let mut buf = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

        let bytes_written = {
            let mut writable = &mut buf[..];
            leb128::write::unsigned(&mut writable, 4097).expect("should write ok")
        };

        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_addrx;
        let value = AttributeValue::DebugAddrIndex(DebugAddrIndex(4097));
        test_parse_attribute(&buf, bytes_written, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_addrx1() {
        let buf = [0x01, 0x99, 0x99];
        let unit = test_parse_attribute_unit(4, Format::Dwarf64, LittleEndian);
        let form = constants::DW_FORM_addrx1;
        let value = AttributeValue::DebugAddrIndex(DebugAddrIndex(0x01));
        test_parse_attribute(&buf, 1, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_addrx2() {
        let buf = [0x01, 0x02, 0x99, 0x99];
        let unit = test_parse_attribute_unit(4, Format::Dwarf64, LittleEndian);
        let form = constants::DW_FORM_addrx2;
        let value = AttributeValue::DebugAddrIndex(DebugAddrIndex(0x0201));
        test_parse_attribute(&buf, 2, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_addrx3() {
        let buf = [0x01, 0x02, 0x03, 0x99, 0x99];
        let unit = test_parse_attribute_unit(4, Format::Dwarf64, LittleEndian);
        let form = constants::DW_FORM_addrx3;
        let value = AttributeValue::DebugAddrIndex(DebugAddrIndex(0x03_0201));
        test_parse_attribute(&buf, 3, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_addrx4() {
        let buf = [0x01, 0x02, 0x03, 0x04, 0x99, 0x99];
        let unit = test_parse_attribute_unit(4, Format::Dwarf64, LittleEndian);
        let form = constants::DW_FORM_addrx4;
        let value = AttributeValue::DebugAddrIndex(DebugAddrIndex(0x0403_0201));
        test_parse_attribute(&buf, 4, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_loclistx() {
        let mut buf = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

        let bytes_written = {
            let mut writable = &mut buf[..];
            leb128::write::unsigned(&mut writable, 4097).expect("should write ok")
        };

        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_loclistx;
        let value = AttributeValue::DebugLocListsIndex(DebugLocListsIndex(4097));
        test_parse_attribute(&buf, bytes_written, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_rnglistx() {
        let mut buf = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

        let bytes_written = {
            let mut writable = &mut buf[..];
            leb128::write::unsigned(&mut writable, 4097).expect("should write ok")
        };

        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_rnglistx;
        let value = AttributeValue::DebugRngListsIndex(DebugRngListsIndex(4097));
        test_parse_attribute(&buf, bytes_written, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_indirect() {
        let mut buf = [0; 100];

        let bytes_written = {
            let mut writable = &mut buf[..];
            leb128::write::unsigned(&mut writable, constants::DW_FORM_udata.0.into())
                .expect("should write udata")
                + leb128::write::unsigned(&mut writable, 9_999_999).expect("should write value")
        };

        let unit = test_parse_attribute_unit_default();
        let form = constants::DW_FORM_indirect;
        let value = AttributeValue::Udata(9_999_999);
        test_parse_attribute(&buf, bytes_written, &unit, form, value);
    }

    #[test]
    fn test_parse_attribute_indirect_implicit_const() {
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 4,
            address_size: 4,
        };
        let mut buf = [0; 100];
        let mut writable = &mut buf[..];
        leb128::write::unsigned(&mut writable, constants::DW_FORM_implicit_const.0.into())
            .expect("should write implicit_const");

        let input = &mut EndianSlice::new(&buf, LittleEndian);
        let spec =
            AttributeSpecification::new(constants::DW_AT_low_pc, constants::DW_FORM_indirect, None);
        assert_eq!(
            parse_attribute(input, encoding, spec),
            Err(Error::InvalidImplicitConst)
        );
    }

    #[test]
    fn test_attrs_iter() {
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 4,
            address_size: 4,
        };
        let unit = UnitHeader::new(
            encoding,
            7,
            UnitType::Compilation,
            DebugAbbrevOffset(0x0807_0605),
            DebugInfoOffset(0).into(),
            EndianSlice::new(&[], LittleEndian),
        );

        let abbrev = Abbreviation::new(
            42,
            constants::DW_TAG_subprogram,
            constants::DW_CHILDREN_yes,
            vec![
                AttributeSpecification::new(constants::DW_AT_name, constants::DW_FORM_string, None),
                AttributeSpecification::new(constants::DW_AT_low_pc, constants::DW_FORM_addr, None),
                AttributeSpecification::new(
                    constants::DW_AT_high_pc,
                    constants::DW_FORM_addr,
                    None,
                ),
            ]
            .into(),
        );

        // "foo", 42, 1337, 4 dangling bytes of 0xaa where children would be
        let buf = [
            0x66, 0x6f, 0x6f, 0x00, 0x2a, 0x00, 0x00, 0x00, 0x39, 0x05, 0x00, 0x00, 0xaa, 0xaa,
            0xaa, 0xaa,
        ];

        let entry = DebuggingInformationEntry {
            offset: UnitOffset(0),
            attrs_slice: EndianSlice::new(&buf, LittleEndian),
            attrs_len: Cell::new(None),
            abbrev: &abbrev,
            unit: &unit,
        };

        let mut attrs = AttrsIter {
            input: EndianSlice::new(&buf, LittleEndian),
            attributes: abbrev.attributes(),
            entry: &entry,
        };

        match attrs.next() {
            Ok(Some(attr)) => {
                assert_eq!(
                    attr,
                    Attribute {
                        name: constants::DW_AT_name,
                        value: AttributeValue::String(EndianSlice::new(b"foo", LittleEndian)),
                    }
                );
            }
            otherwise => {
                panic!("Unexpected parse result = {:#?}", otherwise);
            }
        }

        assert!(entry.attrs_len.get().is_none());

        match attrs.next() {
            Ok(Some(attr)) => {
                assert_eq!(
                    attr,
                    Attribute {
                        name: constants::DW_AT_low_pc,
                        value: AttributeValue::Addr(0x2a),
                    }
                );
            }
            otherwise => {
                panic!("Unexpected parse result = {:#?}", otherwise);
            }
        }

        assert!(entry.attrs_len.get().is_none());

        match attrs.next() {
            Ok(Some(attr)) => {
                assert_eq!(
                    attr,
                    Attribute {
                        name: constants::DW_AT_high_pc,
                        value: AttributeValue::Addr(0x539),
                    }
                );
            }
            otherwise => {
                panic!("Unexpected parse result = {:#?}", otherwise);
            }
        }

        assert!(entry.attrs_len.get().is_none());

        assert!(attrs.next().expect("should parse next").is_none());
        assert!(entry.attrs_len.get().is_some());
        assert_eq!(
            entry.attrs_len.get().expect("should have entry.attrs_len"),
            buf.len() - 4
        )
    }

    #[test]
    fn test_attrs_iter_incomplete() {
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 4,
            address_size: 4,
        };
        let unit = UnitHeader::new(
            encoding,
            7,
            UnitType::Compilation,
            DebugAbbrevOffset(0x0807_0605),
            DebugInfoOffset(0).into(),
            EndianSlice::new(&[], LittleEndian),
        );

        let abbrev = Abbreviation::new(
            42,
            constants::DW_TAG_subprogram,
            constants::DW_CHILDREN_yes,
            vec![
                AttributeSpecification::new(constants::DW_AT_name, constants::DW_FORM_string, None),
                AttributeSpecification::new(constants::DW_AT_low_pc, constants::DW_FORM_addr, None),
                AttributeSpecification::new(
                    constants::DW_AT_high_pc,
                    constants::DW_FORM_addr,
                    None,
                ),
            ]
            .into(),
        );

        // "foo"
        let buf = [0x66, 0x6f, 0x6f, 0x00];

        let entry = DebuggingInformationEntry {
            offset: UnitOffset(0),
            attrs_slice: EndianSlice::new(&buf, LittleEndian),
            attrs_len: Cell::new(None),
            abbrev: &abbrev,
            unit: &unit,
        };

        let mut attrs = AttrsIter {
            input: EndianSlice::new(&buf, LittleEndian),
            attributes: abbrev.attributes(),
            entry: &entry,
        };

        match attrs.next() {
            Ok(Some(attr)) => {
                assert_eq!(
                    attr,
                    Attribute {
                        name: constants::DW_AT_name,
                        value: AttributeValue::String(EndianSlice::new(b"foo", LittleEndian)),
                    }
                );
            }
            otherwise => {
                panic!("Unexpected parse result = {:#?}", otherwise);
            }
        }

        assert!(entry.attrs_len.get().is_none());

        // Return error for incomplete attribute.
        assert!(attrs.next().is_err());
        assert!(entry.attrs_len.get().is_none());

        // Return error for all subsequent calls.
        assert!(attrs.next().is_err());
        assert!(attrs.next().is_err());
        assert!(attrs.next().is_err());
        assert!(attrs.next().is_err());
        assert!(entry.attrs_len.get().is_none());
    }

    fn assert_entry_name<Endian>(
        entry: &DebuggingInformationEntry<'_, '_, EndianSlice<'_, Endian>>,
        name: &str,
    ) where
        Endian: Endianity,
    {
        let value = entry
            .attr_value(constants::DW_AT_name)
            .expect("Should have parsed the name attribute")
            .expect("Should have found the name attribute");

        assert_eq!(
            value,
            AttributeValue::String(EndianSlice::new(name.as_bytes(), Endian::default()))
        );
    }

    fn assert_current_name<Endian>(
        cursor: &EntriesCursor<'_, '_, EndianSlice<'_, Endian>>,
        name: &str,
    ) where
        Endian: Endianity,
    {
        let entry = cursor.current().expect("Should have an entry result");
        assert_entry_name(entry, name);
    }

    fn assert_next_entry<Endian>(
        cursor: &mut EntriesCursor<'_, '_, EndianSlice<'_, Endian>>,
        name: &str,
    ) where
        Endian: Endianity,
    {
        cursor
            .next_entry()
            .expect("Should parse next entry")
            .expect("Should have an entry");
        assert_current_name(cursor, name);
    }

    fn assert_next_entry_null<Endian>(cursor: &mut EntriesCursor<'_, '_, EndianSlice<'_, Endian>>)
    where
        Endian: Endianity,
    {
        cursor
            .next_entry()
            .expect("Should parse next entry")
            .expect("Should have an entry");
        assert!(cursor.current().is_none());
    }

    fn assert_next_dfs<Endian>(
        cursor: &mut EntriesCursor<'_, '_, EndianSlice<'_, Endian>>,
        name: &str,
        depth: isize,
    ) where
        Endian: Endianity,
    {
        {
            let (val, entry) = cursor
                .next_dfs()
                .expect("Should parse next dfs")
                .expect("Should not be done with traversal");
            assert_eq!(val, depth);
            assert_entry_name(entry, name);
        }
        assert_current_name(cursor, name);
    }

    fn assert_next_sibling<Endian>(
        cursor: &mut EntriesCursor<'_, '_, EndianSlice<'_, Endian>>,
        name: &str,
    ) where
        Endian: Endianity,
    {
        {
            let entry = cursor
                .next_sibling()
                .expect("Should parse next sibling")
                .expect("Should not be done with traversal");
            assert_entry_name(entry, name);
        }
        assert_current_name(cursor, name);
    }

    fn assert_valid_sibling_ptr<Endian>(cursor: &EntriesCursor<'_, '_, EndianSlice<'_, Endian>>)
    where
        Endian: Endianity,
    {
        let sibling_ptr = cursor
            .current()
            .expect("Should have current entry")
            .attr_value(constants::DW_AT_sibling);
        match sibling_ptr {
            Ok(Some(AttributeValue::UnitRef(offset))) => {
                cursor
                    .unit
                    .range_from(offset..)
                    .expect("Sibling offset should be valid");
            }
            _ => panic!("Invalid sibling pointer {:?}", sibling_ptr),
        }
    }

    fn entries_cursor_tests_abbrev_buf() -> Vec<u8> {
        #[rustfmt::skip]
        let section = Section::with_endian(Endian::Little)
            .abbrev(1, DW_TAG_subprogram, DW_CHILDREN_yes)
                .abbrev_attr(DW_AT_name, DW_FORM_string)
                .abbrev_attr_null()
            .abbrev_null();
        section.get_contents().unwrap()
    }

    fn entries_cursor_tests_debug_info_buf() -> Vec<u8> {
        #[rustfmt::skip]
        let section = Section::with_endian(Endian::Little)
            .die(1, |s| s.attr_string("001"))
                .die(1, |s| s.attr_string("002"))
                    .die(1, |s| s.attr_string("003"))
                        .die_null()
                    .die_null()
                .die(1, |s| s.attr_string("004"))
                    .die(1, |s| s.attr_string("005"))
                        .die_null()
                    .die(1, |s| s.attr_string("006"))
                        .die_null()
                    .die_null()
                .die(1, |s| s.attr_string("007"))
                    .die(1, |s| s.attr_string("008"))
                        .die(1, |s| s.attr_string("009"))
                            .die_null()
                        .die_null()
                    .die_null()
                .die(1, |s| s.attr_string("010"))
                    .die_null()
                .die_null();
        let entries_buf = section.get_contents().unwrap();

        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 4,
            address_size: 4,
        };
        let mut unit = UnitHeader {
            encoding,
            unit_length: 0,
            unit_type: UnitType::Compilation,
            debug_abbrev_offset: DebugAbbrevOffset(0),
            unit_offset: DebugInfoOffset(0).into(),
            entries_buf: EndianSlice::new(&entries_buf, LittleEndian),
        };
        let section = Section::with_endian(Endian::Little).unit(&mut unit);
        section.get_contents().unwrap()
    }

    #[test]
    fn test_cursor_next_entry_incomplete() {
        #[rustfmt::skip]
        let section = Section::with_endian(Endian::Little)
            .die(1, |s| s.attr_string("001"))
                .die(1, |s| s.attr_string("002"))
                    .die(1, |s| s);
        let entries_buf = section.get_contents().unwrap();

        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 4,
            address_size: 4,
        };
        let mut unit = UnitHeader {
            encoding,
            unit_length: 0,
            unit_type: UnitType::Compilation,
            debug_abbrev_offset: DebugAbbrevOffset(0),
            unit_offset: DebugInfoOffset(0).into(),
            entries_buf: EndianSlice::new(&entries_buf, LittleEndian),
        };
        let section = Section::with_endian(Endian::Little).unit(&mut unit);
        let info_buf = &section.get_contents().unwrap();
        let debug_info = DebugInfo::new(info_buf, LittleEndian);

        let unit = debug_info
            .units()
            .next()
            .expect("should have a unit result")
            .expect("and it should be ok");

        let abbrevs_buf = &entries_cursor_tests_abbrev_buf();
        let debug_abbrev = DebugAbbrev::new(abbrevs_buf, LittleEndian);

        let abbrevs = unit
            .abbreviations(&debug_abbrev)
            .expect("Should parse abbreviations");

        let mut cursor = unit.entries(&abbrevs);

        assert_next_entry(&mut cursor, "001");
        assert_next_entry(&mut cursor, "002");

        {
            // Entry code is present, but none of the attributes.
            cursor
                .next_entry()
                .expect("Should parse next entry")
                .expect("Should have an entry");
            let entry = cursor.current().expect("Should have an entry result");
            assert!(entry.attrs().next().is_err());
        }

        assert!(cursor.next_entry().is_err());
        assert!(cursor.next_entry().is_err());
    }

    #[test]
    fn test_cursor_next_entry() {
        let info_buf = &entries_cursor_tests_debug_info_buf();
        let debug_info = DebugInfo::new(info_buf, LittleEndian);

        let unit = debug_info
            .units()
            .next()
            .expect("should have a unit result")
            .expect("and it should be ok");

        let abbrevs_buf = &entries_cursor_tests_abbrev_buf();
        let debug_abbrev = DebugAbbrev::new(abbrevs_buf, LittleEndian);

        let abbrevs = unit
            .abbreviations(&debug_abbrev)
            .expect("Should parse abbreviations");

        let mut cursor = unit.entries(&abbrevs);

        assert_next_entry(&mut cursor, "001");
        assert_next_entry(&mut cursor, "002");
        assert_next_entry(&mut cursor, "003");
        assert_next_entry_null(&mut cursor);
        assert_next_entry_null(&mut cursor);
        assert_next_entry(&mut cursor, "004");
        assert_next_entry(&mut cursor, "005");
        assert_next_entry_null(&mut cursor);
        assert_next_entry(&mut cursor, "006");
        assert_next_entry_null(&mut cursor);
        assert_next_entry_null(&mut cursor);
        assert_next_entry(&mut cursor, "007");
        assert_next_entry(&mut cursor, "008");
        assert_next_entry(&mut cursor, "009");
        assert_next_entry_null(&mut cursor);
        assert_next_entry_null(&mut cursor);
        assert_next_entry_null(&mut cursor);
        assert_next_entry(&mut cursor, "010");
        assert_next_entry_null(&mut cursor);
        assert_next_entry_null(&mut cursor);

        assert!(cursor
            .next_entry()
            .expect("Should parse next entry")
            .is_none());
        assert!(cursor.current().is_none());
    }

    #[test]
    fn test_cursor_next_dfs() {
        let info_buf = &entries_cursor_tests_debug_info_buf();
        let debug_info = DebugInfo::new(info_buf, LittleEndian);

        let unit = debug_info
            .units()
            .next()
            .expect("should have a unit result")
            .expect("and it should be ok");

        let abbrevs_buf = &entries_cursor_tests_abbrev_buf();
        let debug_abbrev = DebugAbbrev::new(abbrevs_buf, LittleEndian);

        let abbrevs = unit
            .abbreviations(&debug_abbrev)
            .expect("Should parse abbreviations");

        let mut cursor = unit.entries(&abbrevs);

        assert_next_dfs(&mut cursor, "001", 0);
        assert_next_dfs(&mut cursor, "002", 1);
        assert_next_dfs(&mut cursor, "003", 1);
        assert_next_dfs(&mut cursor, "004", -1);
        assert_next_dfs(&mut cursor, "005", 1);
        assert_next_dfs(&mut cursor, "006", 0);
        assert_next_dfs(&mut cursor, "007", -1);
        assert_next_dfs(&mut cursor, "008", 1);
        assert_next_dfs(&mut cursor, "009", 1);
        assert_next_dfs(&mut cursor, "010", -2);

        assert!(cursor.next_dfs().expect("Should parse next dfs").is_none());
        assert!(cursor.current().is_none());
    }

    #[test]
    fn test_cursor_next_sibling_no_sibling_ptr() {
        let info_buf = &entries_cursor_tests_debug_info_buf();
        let debug_info = DebugInfo::new(info_buf, LittleEndian);

        let unit = debug_info
            .units()
            .next()
            .expect("should have a unit result")
            .expect("and it should be ok");

        let abbrevs_buf = &entries_cursor_tests_abbrev_buf();
        let debug_abbrev = DebugAbbrev::new(abbrevs_buf, LittleEndian);

        let abbrevs = unit
            .abbreviations(&debug_abbrev)
            .expect("Should parse abbreviations");

        let mut cursor = unit.entries(&abbrevs);

        assert_next_dfs(&mut cursor, "001", 0);

        // Down to the first child of the root entry.

        assert_next_dfs(&mut cursor, "002", 1);

        // Now iterate all children of the root via `next_sibling`.

        assert_next_sibling(&mut cursor, "004");
        assert_next_sibling(&mut cursor, "007");
        assert_next_sibling(&mut cursor, "010");

        // There should be no more siblings.

        assert!(cursor
            .next_sibling()
            .expect("Should parse next sibling")
            .is_none());
        assert!(cursor.current().is_none());
    }

    #[test]
    fn test_cursor_next_sibling_continuation() {
        let info_buf = &entries_cursor_tests_debug_info_buf();
        let debug_info = DebugInfo::new(info_buf, LittleEndian);

        let unit = debug_info
            .units()
            .next()
            .expect("should have a unit result")
            .expect("and it should be ok");

        let abbrevs_buf = &entries_cursor_tests_abbrev_buf();
        let debug_abbrev = DebugAbbrev::new(abbrevs_buf, LittleEndian);

        let abbrevs = unit
            .abbreviations(&debug_abbrev)
            .expect("Should parse abbreviations");

        let mut cursor = unit.entries(&abbrevs);

        assert_next_dfs(&mut cursor, "001", 0);

        // Down to the first child of the root entry.

        assert_next_dfs(&mut cursor, "002", 1);

        // Get the next sibling, then iterate its children

        assert_next_sibling(&mut cursor, "004");
        assert_next_dfs(&mut cursor, "005", 1);
        assert_next_sibling(&mut cursor, "006");
        assert!(cursor
            .next_sibling()
            .expect("Should parse next sibling")
            .is_none());
        assert!(cursor
            .next_sibling()
            .expect("Should parse next sibling")
            .is_none());
        assert!(cursor
            .next_sibling()
            .expect("Should parse next sibling")
            .is_none());
        assert!(cursor
            .next_sibling()
            .expect("Should parse next sibling")
            .is_none());

        // And we should be able to continue with the children of the root entry.

        assert_next_dfs(&mut cursor, "007", -1);
        assert_next_sibling(&mut cursor, "010");

        // There should be no more siblings.

        assert!(cursor
            .next_sibling()
            .expect("Should parse next sibling")
            .is_none());
        assert!(cursor.current().is_none());
    }

    fn entries_cursor_sibling_abbrev_buf() -> Vec<u8> {
        #[rustfmt::skip]
        let section = Section::with_endian(Endian::Little)
            .abbrev(1, DW_TAG_subprogram, DW_CHILDREN_yes)
                .abbrev_attr(DW_AT_name, DW_FORM_string)
                .abbrev_attr(DW_AT_sibling, DW_FORM_ref1)
                .abbrev_attr_null()
            .abbrev(2, DW_TAG_subprogram, DW_CHILDREN_yes)
                .abbrev_attr(DW_AT_name, DW_FORM_string)
                .abbrev_attr_null()
                .abbrev_null();
        section.get_contents().unwrap()
    }

    fn entries_cursor_sibling_entries_buf(header_size: usize) -> Vec<u8> {
        let start = Label::new();
        let sibling004_ref = Label::new();
        let sibling004 = Label::new();
        let sibling009_ref = Label::new();
        let sibling009 = Label::new();

        #[rustfmt::skip]
        let section = Section::with_endian(Endian::Little)
            .mark(&start)
            .die(2, |s| s.attr_string("001"))
                // Valid sibling attribute.
                .die(1, |s| s.attr_string("002").D8(&sibling004_ref))
                    // Invalid code to ensure the sibling attribute was used.
                    .die(10, |s| s.attr_string("003"))
                        .die_null()
                    .die_null()
                .mark(&sibling004)
                // Invalid sibling attribute.
                .die(1, |s| s.attr_string("004").attr_ref1(255))
                    .die(2, |s| s.attr_string("005"))
                        .die_null()
                    .die_null()
                // Sibling attribute in child only.
                .die(2, |s| s.attr_string("006"))
                    // Valid sibling attribute.
                    .die(1, |s| s.attr_string("007").D8(&sibling009_ref))
                        // Invalid code to ensure the sibling attribute was used.
                        .die(10, |s| s.attr_string("008"))
                            .die_null()
                        .die_null()
                    .mark(&sibling009)
                    .die(2, |s| s.attr_string("009"))
                        .die_null()
                    .die_null()
                // No sibling attribute.
                .die(2, |s| s.attr_string("010"))
                    .die(2, |s| s.attr_string("011"))
                        .die_null()
                    .die_null()
                .die_null();

        let offset = header_size as u64 + (&sibling004 - &start) as u64;
        sibling004_ref.set_const(offset);

        let offset = header_size as u64 + (&sibling009 - &start) as u64;
        sibling009_ref.set_const(offset);

        section.get_contents().unwrap()
    }

    fn test_cursor_next_sibling_with_ptr(
        cursor: &mut EntriesCursor<'_, '_, EndianSlice<'_, LittleEndian>>,
    ) {
        assert_next_dfs(cursor, "001", 0);

        // Down to the first child of the root.

        assert_next_dfs(cursor, "002", 1);

        // Now iterate all children of the root via `next_sibling`.

        assert_valid_sibling_ptr(cursor);
        assert_next_sibling(cursor, "004");
        assert_next_sibling(cursor, "006");
        assert_next_sibling(cursor, "010");

        // There should be no more siblings.

        assert!(cursor
            .next_sibling()
            .expect("Should parse next sibling")
            .is_none());
        assert!(cursor.current().is_none());
    }

    #[test]
    fn test_debug_info_next_sibling_with_ptr() {
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 4,
            address_size: 4,
        };

        let mut unit = UnitHeader {
            encoding,
            unit_length: 0,
            unit_type: UnitType::Compilation,
            debug_abbrev_offset: DebugAbbrevOffset(0),
            unit_offset: DebugInfoOffset(0).into(),
            entries_buf: EndianSlice::new(&[], LittleEndian),
        };
        let header_size = unit.size_of_header();
        let entries_buf = entries_cursor_sibling_entries_buf(header_size);
        unit.entries_buf = EndianSlice::new(&entries_buf, LittleEndian);
        let section = Section::with_endian(Endian::Little).unit(&mut unit);
        let info_buf = section.get_contents().unwrap();
        let debug_info = DebugInfo::new(&info_buf, LittleEndian);

        let unit = debug_info
            .units()
            .next()
            .expect("should have a unit result")
            .expect("and it should be ok");

        let abbrev_buf = entries_cursor_sibling_abbrev_buf();
        let debug_abbrev = DebugAbbrev::new(&abbrev_buf, LittleEndian);

        let abbrevs = unit
            .abbreviations(&debug_abbrev)
            .expect("Should parse abbreviations");

        let mut cursor = unit.entries(&abbrevs);
        test_cursor_next_sibling_with_ptr(&mut cursor);
    }

    #[test]
    fn test_debug_types_next_sibling_with_ptr() {
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 4,
            address_size: 4,
        };
        let mut unit = UnitHeader {
            encoding,
            unit_length: 0,
            unit_type: UnitType::Type {
                type_signature: DebugTypeSignature(0),
                type_offset: UnitOffset(0),
            },
            debug_abbrev_offset: DebugAbbrevOffset(0),
            unit_offset: DebugTypesOffset(0).into(),
            entries_buf: EndianSlice::new(&[], LittleEndian),
        };
        let header_size = unit.size_of_header();
        let entries_buf = entries_cursor_sibling_entries_buf(header_size);
        unit.entries_buf = EndianSlice::new(&entries_buf, LittleEndian);
        let section = Section::with_endian(Endian::Little).unit(&mut unit);
        let info_buf = section.get_contents().unwrap();
        let debug_types = DebugTypes::new(&info_buf, LittleEndian);

        let unit = debug_types
            .units()
            .next()
            .expect("should have a unit result")
            .expect("and it should be ok");

        let abbrev_buf = entries_cursor_sibling_abbrev_buf();
        let debug_abbrev = DebugAbbrev::new(&abbrev_buf, LittleEndian);

        let abbrevs = unit
            .abbreviations(&debug_abbrev)
            .expect("Should parse abbreviations");

        let mut cursor = unit.entries(&abbrevs);
        test_cursor_next_sibling_with_ptr(&mut cursor);
    }

    #[test]
    fn test_entries_at_offset() {
        let info_buf = &entries_cursor_tests_debug_info_buf();
        let debug_info = DebugInfo::new(info_buf, LittleEndian);

        let unit = debug_info
            .units()
            .next()
            .expect("should have a unit result")
            .expect("and it should be ok");

        let abbrevs_buf = &entries_cursor_tests_abbrev_buf();
        let debug_abbrev = DebugAbbrev::new(abbrevs_buf, LittleEndian);

        let abbrevs = unit
            .abbreviations(&debug_abbrev)
            .expect("Should parse abbreviations");

        let mut cursor = unit
            .entries_at_offset(&abbrevs, UnitOffset(unit.header_size()))
            .unwrap();
        assert_next_entry(&mut cursor, "001");

        let cursor = unit.entries_at_offset(&abbrevs, UnitOffset(0));
        match cursor {
            Err(Error::OffsetOutOfBounds) => {}
            otherwise => {
                panic!("Unexpected parse result = {:#?}", otherwise);
            }
        }
    }

    fn entries_tree_tests_debug_abbrevs_buf() -> Vec<u8> {
        #[rustfmt::skip]
        let section = Section::with_endian(Endian::Little)
            .abbrev(1, DW_TAG_subprogram, DW_CHILDREN_yes)
                .abbrev_attr(DW_AT_name, DW_FORM_string)
                .abbrev_attr_null()
            .abbrev(2, DW_TAG_subprogram, DW_CHILDREN_no)
                .abbrev_attr(DW_AT_name, DW_FORM_string)
                .abbrev_attr_null()
            .abbrev_null()
            .get_contents()
            .unwrap();
        section
    }

    fn entries_tree_tests_debug_info_buf(header_size: usize) -> (Vec<u8>, UnitOffset) {
        let start = Label::new();
        let entry2 = Label::new();
        #[rustfmt::skip]
        let section = Section::with_endian(Endian::Little)
            .mark(&start)
            .die(1, |s| s.attr_string("root"))
                .die(1, |s| s.attr_string("1"))
                    .die(1, |s| s.attr_string("1a"))
                        .die_null()
                    .die(2, |s| s.attr_string("1b"))
                    .die_null()
                .mark(&entry2)
                .die(1, |s| s.attr_string("2"))
                    .die(1, |s| s.attr_string("2a"))
                        .die(1, |s| s.attr_string("2a1"))
                            .die_null()
                        .die_null()
                    .die(1, |s| s.attr_string("2b"))
                        .die(2, |s| s.attr_string("2b1"))
                        .die_null()
                    .die_null()
                .die(1, |s| s.attr_string("3"))
                    .die(1, |s| s.attr_string("3a"))
                        .die(2, |s| s.attr_string("3a1"))
                        .die(2, |s| s.attr_string("3a2"))
                        .die_null()
                    .die(2, |s| s.attr_string("3b"))
                    .die_null()
                .die(2, |s| s.attr_string("final"))
                .die_null()
            .get_contents()
            .unwrap();
        let entry2 = UnitOffset(header_size + (&entry2 - &start) as usize);
        (section, entry2)
    }

    #[test]
    fn test_entries_tree() {
        fn assert_entry<'input, 'abbrev, 'unit, 'tree, Endian>(
            node: Result<
                Option<EntriesTreeNode<'abbrev, 'unit, 'tree, EndianSlice<'input, Endian>>>,
            >,
            name: &str,
        ) -> EntriesTreeIter<'abbrev, 'unit, 'tree, EndianSlice<'input, Endian>>
        where
            Endian: Endianity,
        {
            let node = node
                .expect("Should parse entry")
                .expect("Should have entry");
            assert_entry_name(node.entry(), name);
            node.children()
        }

        fn assert_null<E: Endianity>(
            node: Result<Option<EntriesTreeNode<'_, '_, '_, EndianSlice<'_, E>>>>,
        ) {
            match node {
                Ok(None) => {}
                otherwise => {
                    panic!("Unexpected parse result = {:#?}", otherwise);
                }
            }
        }

        let abbrevs_buf = entries_tree_tests_debug_abbrevs_buf();
        let debug_abbrev = DebugAbbrev::new(&abbrevs_buf, LittleEndian);

        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 4,
            address_size: 4,
        };
        let mut unit = UnitHeader {
            encoding,
            unit_length: 0,
            unit_type: UnitType::Compilation,
            debug_abbrev_offset: DebugAbbrevOffset(0),
            unit_offset: DebugInfoOffset(0).into(),
            entries_buf: EndianSlice::new(&[], LittleEndian),
        };
        let header_size = unit.size_of_header();
        let (entries_buf, entry2) = entries_tree_tests_debug_info_buf(header_size);
        unit.entries_buf = EndianSlice::new(&entries_buf, LittleEndian);
        let info_buf = Section::with_endian(Endian::Little)
            .unit(&mut unit)
            .get_contents()
            .unwrap();
        let debug_info = DebugInfo::new(&info_buf, LittleEndian);

        let unit = debug_info
            .units()
            .next()
            .expect("Should parse unit")
            .expect("and it should be some");
        let abbrevs = unit
            .abbreviations(&debug_abbrev)
            .expect("Should parse abbreviations");
        let mut tree = unit
            .entries_tree(&abbrevs, None)
            .expect("Should have entries tree");

        // Test we can restart iteration of the tree.
        {
            let mut iter = assert_entry(tree.root().map(Some), "root");
            assert_entry(iter.next(), "1");
        }
        {
            let mut iter = assert_entry(tree.root().map(Some), "root");
            assert_entry(iter.next(), "1");
        }

        let mut iter = assert_entry(tree.root().map(Some), "root");
        {
            // Test iteration with children.
            let mut iter = assert_entry(iter.next(), "1");
            {
                // Test iteration with children flag, but no children.
                let mut iter = assert_entry(iter.next(), "1a");
                assert_null(iter.next());
                assert_null(iter.next());
            }
            {
                // Test iteration without children flag.
                let mut iter = assert_entry(iter.next(), "1b");
                assert_null(iter.next());
                assert_null(iter.next());
            }
            assert_null(iter.next());
            assert_null(iter.next());
        }
        {
            // Test skipping over children.
            let mut iter = assert_entry(iter.next(), "2");
            assert_entry(iter.next(), "2a");
            assert_entry(iter.next(), "2b");
            assert_null(iter.next());
        }
        {
            // Test skipping after partial iteration.
            let mut iter = assert_entry(iter.next(), "3");
            {
                let mut iter = assert_entry(iter.next(), "3a");
                assert_entry(iter.next(), "3a1");
                // Parent iter should be able to skip over "3a2".
            }
            assert_entry(iter.next(), "3b");
            assert_null(iter.next());
        }
        assert_entry(iter.next(), "final");
        assert_null(iter.next());

        // Test starting at an offset.
        let mut tree = unit
            .entries_tree(&abbrevs, Some(entry2))
            .expect("Should have entries tree");
        let mut iter = assert_entry(tree.root().map(Some), "2");
        assert_entry(iter.next(), "2a");
        assert_entry(iter.next(), "2b");
        assert_null(iter.next());
    }

    #[test]
    fn test_entries_raw() {
        fn assert_abbrev<'abbrev, Endian>(
            entries: &mut EntriesRaw<'abbrev, '_, EndianSlice<'_, Endian>>,
            tag: DwTag,
        ) -> &'abbrev Abbreviation
        where
            Endian: Endianity,
        {
            let abbrev = entries
                .read_abbreviation()
                .expect("Should parse abbrev")
                .expect("Should have abbrev");
            assert_eq!(abbrev.tag(), tag);
            abbrev
        }

        fn assert_null<Endian>(entries: &mut EntriesRaw<'_, '_, EndianSlice<'_, Endian>>)
        where
            Endian: Endianity,
        {
            match entries.read_abbreviation() {
                Ok(None) => {}
                otherwise => {
                    panic!("Unexpected parse result = {:#?}", otherwise);
                }
            }
        }

        fn assert_attr<Endian>(
            entries: &mut EntriesRaw<'_, '_, EndianSlice<'_, Endian>>,
            spec: Option<AttributeSpecification>,
            name: DwAt,
            value: &str,
        ) where
            Endian: Endianity,
        {
            let spec = spec.expect("Should have attribute specification");
            let attr = entries
                .read_attribute(spec)
                .expect("Should parse attribute");
            assert_eq!(attr.name(), name);
            assert_eq!(
                attr.value(),
                AttributeValue::String(EndianSlice::new(value.as_bytes(), Endian::default()))
            );
        }

        #[rustfmt::skip]
        let section = Section::with_endian(Endian::Little)
            .abbrev(1, DW_TAG_subprogram, DW_CHILDREN_yes)
                .abbrev_attr(DW_AT_name, DW_FORM_string)
                .abbrev_attr(DW_AT_linkage_name, DW_FORM_string)
                .abbrev_attr_null()
            .abbrev(2, DW_TAG_variable, DW_CHILDREN_no)
                .abbrev_attr(DW_AT_name, DW_FORM_string)
                .abbrev_attr_null()
            .abbrev_null();
        let abbrevs_buf = section.get_contents().unwrap();
        let debug_abbrev = DebugAbbrev::new(&abbrevs_buf, LittleEndian);

        #[rustfmt::skip]
        let section = Section::with_endian(Endian::Little)
            .die(1, |s| s.attr_string("f1").attr_string("l1"))
                .die(2, |s| s.attr_string("v1"))
                .die(2, |s| s.attr_string("v2"))
                .die(1, |s| s.attr_string("f2").attr_string("l2"))
                    .die_null()
                .die_null();
        let entries_buf = section.get_contents().unwrap();

        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 4,
            address_size: 4,
        };
        let mut unit = UnitHeader {
            encoding,
            unit_length: 0,
            unit_type: UnitType::Compilation,
            debug_abbrev_offset: DebugAbbrevOffset(0),
            unit_offset: DebugInfoOffset(0).into(),
            entries_buf: EndianSlice::new(&entries_buf, LittleEndian),
        };
        let section = Section::with_endian(Endian::Little).unit(&mut unit);
        let info_buf = section.get_contents().unwrap();
        let debug_info = DebugInfo::new(&info_buf, LittleEndian);

        let unit = debug_info
            .units()
            .next()
            .expect("should have a unit result")
            .expect("and it should be ok");

        let abbrevs = unit
            .abbreviations(&debug_abbrev)
            .expect("Should parse abbreviations");

        let mut entries = unit
            .entries_raw(&abbrevs, None)
            .expect("Should have entries");

        assert_eq!(entries.next_depth(), 0);
        let abbrev = assert_abbrev(&mut entries, DW_TAG_subprogram);
        let mut attrs = abbrev.attributes().iter().copied();
        assert_attr(&mut entries, attrs.next(), DW_AT_name, "f1");
        assert_attr(&mut entries, attrs.next(), DW_AT_linkage_name, "l1");
        assert!(attrs.next().is_none());

        assert_eq!(entries.next_depth(), 1);
        let abbrev = assert_abbrev(&mut entries, DW_TAG_variable);
        let mut attrs = abbrev.attributes().iter().copied();
        assert_attr(&mut entries, attrs.next(), DW_AT_name, "v1");
        assert!(attrs.next().is_none());

        assert_eq!(entries.next_depth(), 1);
        let abbrev = assert_abbrev(&mut entries, DW_TAG_variable);
        let mut attrs = abbrev.attributes().iter().copied();
        assert_attr(&mut entries, attrs.next(), DW_AT_name, "v2");
        assert!(attrs.next().is_none());

        assert_eq!(entries.next_depth(), 1);
        let abbrev = assert_abbrev(&mut entries, DW_TAG_subprogram);
        let mut attrs = abbrev.attributes().iter().copied();
        assert_attr(&mut entries, attrs.next(), DW_AT_name, "f2");
        assert_attr(&mut entries, attrs.next(), DW_AT_linkage_name, "l2");
        assert!(attrs.next().is_none());

        assert_eq!(entries.next_depth(), 2);
        assert_null(&mut entries);

        assert_eq!(entries.next_depth(), 1);
        assert_null(&mut entries);

        assert_eq!(entries.next_depth(), 0);
        assert!(entries.is_empty());
    }

    #[test]
    fn test_debug_info_offset() {
        let padding = &[0; 10];
        let entries = &[0; 20];
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 4,
            address_size: 4,
        };
        let mut unit = UnitHeader {
            encoding,
            unit_length: 0,
            unit_type: UnitType::Compilation,
            debug_abbrev_offset: DebugAbbrevOffset(0),
            unit_offset: DebugInfoOffset(0).into(),
            entries_buf: EndianSlice::new(entries, LittleEndian),
        };
        Section::with_endian(Endian::Little)
            .append_bytes(padding)
            .unit(&mut unit);
        let offset = padding.len();
        let header_length = unit.size_of_header();
        let length = unit.length_including_self();
        assert_eq!(DebugInfoOffset(0).to_unit_offset(&unit), None);
        assert_eq!(DebugInfoOffset(offset - 1).to_unit_offset(&unit), None);
        assert_eq!(DebugInfoOffset(offset).to_unit_offset(&unit), None);
        assert_eq!(
            DebugInfoOffset(offset + header_length - 1).to_unit_offset(&unit),
            None
        );
        assert_eq!(
            DebugInfoOffset(offset + header_length).to_unit_offset(&unit),
            Some(UnitOffset(header_length))
        );
        assert_eq!(
            DebugInfoOffset(offset + length - 1).to_unit_offset(&unit),
            Some(UnitOffset(length - 1))
        );
        assert_eq!(DebugInfoOffset(offset + length).to_unit_offset(&unit), None);
        assert_eq!(
            UnitOffset(header_length).to_debug_info_offset(&unit),
            Some(DebugInfoOffset(offset + header_length))
        );
        assert_eq!(
            UnitOffset(length - 1).to_debug_info_offset(&unit),
            Some(DebugInfoOffset(offset + length - 1))
        );
    }

    #[test]
    fn test_debug_types_offset() {
        let padding = &[0; 10];
        let entries = &[0; 20];
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 4,
            address_size: 4,
        };
        let mut unit = UnitHeader {
            encoding,
            unit_length: 0,
            unit_type: UnitType::Type {
                type_signature: DebugTypeSignature(0),
                type_offset: UnitOffset(0),
            },
            debug_abbrev_offset: DebugAbbrevOffset(0),
            unit_offset: DebugTypesOffset(0).into(),
            entries_buf: EndianSlice::new(entries, LittleEndian),
        };
        Section::with_endian(Endian::Little)
            .append_bytes(padding)
            .unit(&mut unit);
        let offset = padding.len();
        let header_length = unit.size_of_header();
        let length = unit.length_including_self();
        assert_eq!(DebugTypesOffset(0).to_unit_offset(&unit), None);
        assert_eq!(DebugTypesOffset(offset - 1).to_unit_offset(&unit), None);
        assert_eq!(DebugTypesOffset(offset).to_unit_offset(&unit), None);
        assert_eq!(
            DebugTypesOffset(offset + header_length - 1).to_unit_offset(&unit),
            None
        );
        assert_eq!(
            DebugTypesOffset(offset + header_length).to_unit_offset(&unit),
            Some(UnitOffset(header_length))
        );
        assert_eq!(
            DebugTypesOffset(offset + length - 1).to_unit_offset(&unit),
            Some(UnitOffset(length - 1))
        );
        assert_eq!(
            DebugTypesOffset(offset + length).to_unit_offset(&unit),
            None
        );
        assert_eq!(
            UnitOffset(header_length).to_debug_types_offset(&unit),
            Some(DebugTypesOffset(offset + header_length))
        );
        assert_eq!(
            UnitOffset(length - 1).to_debug_types_offset(&unit),
            Some(DebugTypesOffset(offset + length - 1))
        );
    }

    #[test]
    fn test_length_including_self() {
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 4,
            address_size: 4,
        };
        let mut unit = UnitHeader {
            encoding,
            unit_length: 0,
            unit_type: UnitType::Compilation,
            debug_abbrev_offset: DebugAbbrevOffset(0),
            unit_offset: DebugInfoOffset(0).into(),
            entries_buf: EndianSlice::new(&[], LittleEndian),
        };
        unit.encoding.format = Format::Dwarf32;
        assert_eq!(unit.length_including_self(), 4);
        unit.encoding.format = Format::Dwarf64;
        assert_eq!(unit.length_including_self(), 12);
        unit.unit_length = 10;
        assert_eq!(unit.length_including_self(), 22);
    }

    #[test]
    fn test_parse_type_unit_abbrevs() {
        let types_buf = [
            // Type unit header
            0x25, 0x00, 0x00, 0x00, // 32-bit unit length = 37
            0x04, 0x00, // Version 4
            0x00, 0x00, 0x00, 0x00, // debug_abbrev_offset
            0x04, // Address size
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, // Type signature
            0x01, 0x02, 0x03, 0x04, // Type offset
            // DIEs
            // Abbreviation code
            0x01, // Attribute of form DW_FORM_string = "foo\0"
            0x66, 0x6f, 0x6f, 0x00, // Children
            // Abbreviation code
            0x01, // Attribute of form DW_FORM_string = "foo\0"
            0x66, 0x6f, 0x6f, 0x00, // Children
            // Abbreviation code
            0x01, // Attribute of form DW_FORM_string = "foo\0"
            0x66, 0x6f, 0x6f, 0x00, // Children
            0x00, // End of children
            0x00, // End of children
            0x00, // End of children
        ];
        let debug_types = DebugTypes::new(&types_buf, LittleEndian);

        let abbrev_buf = [
            // Code
            0x01, // DW_TAG_subprogram
            0x2e, // DW_CHILDREN_yes
            0x01, // Begin attributes
            0x03, // Attribute name = DW_AT_name
            0x08, // Attribute form = DW_FORM_string
            0x00, 0x00, // End attributes
            0x00, // Null terminator
        ];

        let get_some_type_unit = || debug_types.units().next().unwrap().unwrap();

        let unit = get_some_type_unit();

        let read_debug_abbrev_section_somehow = || &abbrev_buf;
        let debug_abbrev = DebugAbbrev::new(read_debug_abbrev_section_somehow(), LittleEndian);
        let _abbrevs_for_unit = unit.abbreviations(&debug_abbrev).unwrap();
    }
}
