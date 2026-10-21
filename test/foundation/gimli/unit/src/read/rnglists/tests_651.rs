#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::Format;
    use crate::constants::*;
    use crate::endianity::LittleEndian;
    use crate::test_util::GimliSectionMethods;
    use alloc::vec::Vec;
    use test_assembler::{Endian, Label, LabelMaker, Section};

    #[test]
    fn test_rnglists() {
        let format = Format::Dwarf32;
        for size in [4, 8] {
            let tombstone = u64::ones_sized(size);
            let tombstone_0 = 0;
            let encoding = Encoding {
                format,
                version: 5,
                address_size: size,
            };
            let section = Section::with_endian(Endian::Little)
                .word(size, 0x0300_0000)
                .word(size, 0x0301_0300)
                .word(size, 0x0301_0400)
                .word(size, 0x0301_0500)
                .word(size, tombstone)
                .word(size, 0x0301_0600)
                .word(size, tombstone_0);
            let buf = section.get_contents().unwrap();
            let debug_addr = &DebugAddr::from(EndianSlice::new(&buf, LittleEndian));
            let debug_addr_base = DebugAddrBase(0);

            let length = Label::new();
            let start = Label::new();
            let first = Label::new();
            let end = Label::new();
            let mut section = Section::with_endian(Endian::Little)
                .initial_length(format, &length, &start)
                .L16(encoding.version)
                .L8(encoding.address_size)
                .L8(0)
                .L32(0)
                .mark(&first);

            let mut expected_ranges = Vec::new();
            let mut expect_range = |begin, end| {
                expected_ranges.push(Range { begin, end });
            };

            // An offset pair using the unit base address.
            section = section.L8(DW_RLE_offset_pair.0).uleb(0x10200).uleb(0x10300);
            expect_range(0x0101_0200, 0x0101_0300);

            section = section.L8(DW_RLE_base_address.0).word(size, 0x0200_0000);
            section = section.L8(DW_RLE_offset_pair.0).uleb(0x10400).uleb(0x10500);
            expect_range(0x0201_0400, 0x0201_0500);

            section = section
                .L8(DW_RLE_start_end.0)
                .word(size, 0x201_0a00)
                .word(size, 0x201_0b00);
            expect_range(0x0201_0a00, 0x0201_0b00);

            section = section
                .L8(DW_RLE_start_length.0)
                .word(size, 0x201_0c00)
                .uleb(0x100);
            expect_range(0x0201_0c00, 0x0201_0d00);

            // An offset pair that starts at 0.
            section = section.L8(DW_RLE_base_address.0).word(size, 0);
            section = section.L8(DW_RLE_offset_pair.0).uleb(0).uleb(1);
            expect_range(0, 1);

            // An offset pair that ends at -1.
            section = section.L8(DW_RLE_base_address.0).word(size, 0);
            section = section.L8(DW_RLE_offset_pair.0).uleb(0).uleb(tombstone);
            expect_range(0, tombstone);

            section = section.L8(DW_RLE_base_addressx.0).uleb(0);
            section = section.L8(DW_RLE_offset_pair.0).uleb(0x10100).uleb(0x10200);
            expect_range(0x0301_0100, 0x0301_0200);

            section = section.L8(DW_RLE_startx_endx.0).uleb(1).uleb(2);
            expect_range(0x0301_0300, 0x0301_0400);

            section = section.L8(DW_RLE_startx_length.0).uleb(3).uleb(0x100);
            expect_range(0x0301_0500, 0x0301_0600);

            // Tombstone entries, all of which should be ignored.
            section = section.L8(DW_RLE_base_addressx.0).uleb(4);
            section = section.L8(DW_RLE_offset_pair.0).uleb(0x11100).uleb(0x11200);

            section = section.L8(DW_RLE_base_address.0).word(size, tombstone);
            section = section.L8(DW_RLE_offset_pair.0).uleb(0x11300).uleb(0x11400);

            section = section.L8(DW_RLE_startx_endx.0).uleb(4).uleb(5);
            section = section.L8(DW_RLE_startx_length.0).uleb(4).uleb(0x100);
            section = section
                .L8(DW_RLE_start_end.0)
                .word(size, tombstone)
                .word(size, 0x201_1500);
            section = section
                .L8(DW_RLE_start_length.0)
                .word(size, tombstone)
                .uleb(0x100);

            // Ignore some instances of 0 for tombstone.
            section = section.L8(DW_RLE_startx_endx.0).uleb(6).uleb(6);
            section = section
                .L8(DW_RLE_start_end.0)
                .word(size, tombstone_0)
                .word(size, tombstone_0);

            // Ignore empty ranges.
            section = section.L8(DW_RLE_base_address.0).word(size, 0);
            section = section.L8(DW_RLE_offset_pair.0).uleb(0).uleb(0);
            section = section.L8(DW_RLE_base_address.0).word(size, 0x10000);
            section = section.L8(DW_RLE_offset_pair.0).uleb(0x1234).uleb(0x1234);

            // A valid range after the tombstones.
            section = section
                .L8(DW_RLE_start_end.0)
                .word(size, 0x201_1600)
                .word(size, 0x201_1700);
            expect_range(0x0201_1600, 0x0201_1700);

            section = section.L8(DW_RLE_end_of_list.0);
            section = section.mark(&end);
            // Some extra data.
            section = section.word(size, 0x1234_5678);
            length.set_const((&end - &start) as u64);

            let offset = RangeListsOffset((&first - &section.start()) as usize);
            let buf = section.get_contents().unwrap();
            let debug_ranges = DebugRanges::new(&[], LittleEndian);
            let debug_rnglists = DebugRngLists::new(&buf, LittleEndian);
            let rnglists = RangeLists::new(debug_ranges, debug_rnglists);
            let mut ranges = rnglists
                .ranges(offset, encoding, 0x0100_0000, debug_addr, debug_addr_base)
                .unwrap();

            for expected_range in expected_ranges {
                let range = ranges.next();
                assert_eq!(
                    range,
                    Ok(Some(expected_range)),
                    "read {:x?}, expect {:x?}",
                    range,
                    expected_range
                );
            }
            assert_eq!(ranges.next(), Ok(None));

            // An offset at the end of buf.
            let mut ranges = rnglists
                .ranges(
                    RangeListsOffset(buf.len()),
                    encoding,
                    0x0100_0000,
                    debug_addr,
                    debug_addr_base,
                )
                .unwrap();
            assert_eq!(ranges.next(), Ok(None));
        }
    }

    #[test]
    fn test_raw_range() {
        let range = RawRange {
            begin: 0,
            end: 0xffff_ffff,
        };
        assert!(!range.is_end());
        assert!(!range.is_base_address(4));
        assert!(!range.is_base_address(8));

        let range = RawRange { begin: 0, end: 0 };
        assert!(range.is_end());
        assert!(!range.is_base_address(4));
        assert!(!range.is_base_address(8));

        let range = RawRange {
            begin: 0xffff_ffff,
            end: 0,
        };
        assert!(!range.is_end());
        assert!(range.is_base_address(4));
        assert!(!range.is_base_address(8));

        let range = RawRange {
            begin: 0xffff_ffff_ffff_ffff,
            end: 0,
        };
        assert!(!range.is_end());
        assert!(!range.is_base_address(4));
        assert!(range.is_base_address(8));
    }

    #[test]
    fn test_ranges() {
        for size in [4, 8] {
            let base = u64::ones_sized(size);
            let tombstone = u64::ones_sized(size) - 1;
            let start = Label::new();
            let first = Label::new();
            let mut section = Section::with_endian(Endian::Little)
                // A range before the offset.
                .mark(&start)
                .word(size, 0x10000)
                .word(size, 0x10100)
                .mark(&first);

            let mut expected_ranges = Vec::new();
            let mut expect_range = |begin, end| {
                expected_ranges.push(Range { begin, end });
            };

            // A normal range.
            section = section.word(size, 0x10200).word(size, 0x10300);
            expect_range(0x0101_0200, 0x0101_0300);
            // A base address selection followed by a normal range.
            section = section.word(size, base).word(size, 0x0200_0000);
            section = section.word(size, 0x10400).word(size, 0x10500);
            expect_range(0x0201_0400, 0x0201_0500);
            // An empty range followed by a normal range.
            section = section.word(size, 0x10600).word(size, 0x10600);
            section = section.word(size, 0x10800).word(size, 0x10900);
            expect_range(0x0201_0800, 0x0201_0900);
            // A range that starts at 0.
            section = section.word(size, base).word(size, 0);
            section = section.word(size, 0).word(size, 1);
            expect_range(0, 1);
            // A range that ends at -1.
            section = section.word(size, base).word(size, 0);
            section = section.word(size, 0).word(size, base);
            expect_range(0, base);
            // A normal range with tombstone.
            section = section.word(size, tombstone).word(size, tombstone);
            // A base address selection with tombstone followed by a normal range.
            section = section.word(size, base).word(size, tombstone);
            section = section.word(size, 0x10a00).word(size, 0x10b00);
            // A range end.
            section = section.word(size, 0).word(size, 0);
            // Some extra data.
            section = section.word(size, 0x1234_5678);

            let buf = section.get_contents().unwrap();
            let debug_ranges = DebugRanges::new(&buf, LittleEndian);
            let debug_rnglists = DebugRngLists::new(&[], LittleEndian);
            let rnglists = RangeLists::new(debug_ranges, debug_rnglists);
            let offset = RangeListsOffset((&first - &start) as usize);
            let debug_addr = &DebugAddr::from(EndianSlice::new(&[], LittleEndian));
            let debug_addr_base = DebugAddrBase(0);
            let encoding = Encoding {
                format: Format::Dwarf32,
                version: 4,
                address_size: size,
            };
            let mut ranges = rnglists
                .ranges(offset, encoding, 0x0100_0000, debug_addr, debug_addr_base)
                .unwrap();

            for expected_range in expected_ranges {
                let range = ranges.next();
                assert_eq!(
                    range,
                    Ok(Some(expected_range)),
                    "read {:x?}, expect {:x?}",
                    range,
                    expected_range
                );
            }
            assert_eq!(ranges.next(), Ok(None));

            // An offset at the end of buf.
            let mut ranges = rnglists
                .ranges(
                    RangeListsOffset(buf.len()),
                    encoding,
                    0x0100_0000,
                    debug_addr,
                    debug_addr_base,
                )
                .unwrap();
            assert_eq!(ranges.next(), Ok(None));
        }
    }

    #[test]
    fn test_ranges_invalid() {
        #[rustfmt::skip]
        let section = Section::with_endian(Endian::Little)
            // An invalid range.
            .L32(0x20000).L32(0x10000)
            // An invalid range after wrapping.
            .L32(0x20000).L32(0xff01_0000);

        let buf = section.get_contents().unwrap();
        let debug_ranges = DebugRanges::new(&buf, LittleEndian);
        let debug_rnglists = DebugRngLists::new(&[], LittleEndian);
        let rnglists = RangeLists::new(debug_ranges, debug_rnglists);
        let debug_addr = &DebugAddr::from(EndianSlice::new(&[], LittleEndian));
        let debug_addr_base = DebugAddrBase(0);
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 4,
            address_size: 4,
        };

        // An invalid range.
        let mut ranges = rnglists
            .ranges(
                RangeListsOffset(0x0),
                encoding,
                0x0100_0000,
                debug_addr,
                debug_addr_base,
            )
            .unwrap();
        assert_eq!(ranges.next(), Ok(None));

        // An invalid range after wrapping.
        let mut ranges = rnglists
            .ranges(
                RangeListsOffset(0x8),
                encoding,
                0x0100_0000,
                debug_addr,
                debug_addr_base,
            )
            .unwrap();
        assert_eq!(ranges.next(), Ok(None));

        // An invalid offset.
        match rnglists.ranges(
            RangeListsOffset(buf.len() + 1),
            encoding,
            0x0100_0000,
            debug_addr,
            debug_addr_base,
        ) {
            Err(Error::UnexpectedEof(_)) => {}
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        }
    }

    #[test]
    fn test_get_offset() {
        for format in [Format::Dwarf32, Format::Dwarf64] {
            let encoding = Encoding {
                format,
                version: 5,
                address_size: 4,
            };

            let zero = Label::new();
            let length = Label::new();
            let start = Label::new();
            let first = Label::new();
            let end = Label::new();
            let mut section = Section::with_endian(Endian::Little)
                .mark(&zero)
                .initial_length(format, &length, &start)
                .D16(encoding.version)
                .D8(encoding.address_size)
                .D8(0)
                .D32(20)
                .mark(&first);
            for i in 0..20 {
                section = section.word(format.word_size(), 1000 + i);
            }
            section = section.mark(&end);
            length.set_const((&end - &start) as u64);
            let section = section.get_contents().unwrap();

            let debug_ranges = DebugRanges::from(EndianSlice::new(&[], LittleEndian));
            let debug_rnglists = DebugRngLists::from(EndianSlice::new(&section, LittleEndian));
            let ranges = RangeLists::new(debug_ranges, debug_rnglists);

            let base = DebugRngListsBase((&first - &zero) as usize);
            assert_eq!(
                ranges.get_offset(encoding, base, DebugRngListsIndex(0)),
                Ok(RangeListsOffset(base.0 + 1000))
            );
            assert_eq!(
                ranges.get_offset(encoding, base, DebugRngListsIndex(19)),
                Ok(RangeListsOffset(base.0 + 1019))
            );
        }
    }
}
