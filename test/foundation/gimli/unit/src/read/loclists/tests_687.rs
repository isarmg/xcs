#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::Format;
    use crate::constants::*;
    use crate::endianity::LittleEndian;
    use crate::read::{EndianSlice, Range};
    use crate::test_util::GimliSectionMethods;
    use alloc::vec::Vec;
    use test_assembler::{Endian, Label, LabelMaker, Section};

    #[test]
    fn test_loclists() {
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

            let mut expected_locations = Vec::new();
            let mut expect_location = |begin, end, data| {
                expected_locations.push(LocationListEntry {
                    range: Range { begin, end },
                    data: Expression(EndianSlice::new(data, LittleEndian)),
                });
            };

            // An offset pair using the unit base address.
            section = section.L8(DW_LLE_offset_pair.0).uleb(0x10200).uleb(0x10300);
            section = section.uleb(4).L32(2);
            expect_location(0x0101_0200, 0x0101_0300, &[2, 0, 0, 0]);

            section = section.L8(DW_LLE_base_address.0).word(size, 0x0200_0000);
            section = section.L8(DW_LLE_offset_pair.0).uleb(0x10400).uleb(0x10500);
            section = section.uleb(4).L32(3);
            expect_location(0x0201_0400, 0x0201_0500, &[3, 0, 0, 0]);

            section = section
                .L8(DW_LLE_start_end.0)
                .word(size, 0x201_0a00)
                .word(size, 0x201_0b00);
            section = section.uleb(4).L32(6);
            expect_location(0x0201_0a00, 0x0201_0b00, &[6, 0, 0, 0]);

            section = section
                .L8(DW_LLE_start_length.0)
                .word(size, 0x201_0c00)
                .uleb(0x100);
            section = section.uleb(4).L32(7);
            expect_location(0x0201_0c00, 0x0201_0d00, &[7, 0, 0, 0]);

            // An offset pair that starts at 0.
            section = section.L8(DW_LLE_base_address.0).word(size, 0);
            section = section.L8(DW_LLE_offset_pair.0).uleb(0).uleb(1);
            section = section.uleb(4).L32(8);
            expect_location(0, 1, &[8, 0, 0, 0]);

            // An offset pair that ends at -1.
            section = section.L8(DW_LLE_base_address.0).word(size, 0);
            section = section.L8(DW_LLE_offset_pair.0).uleb(0).uleb(tombstone);
            section = section.uleb(4).L32(9);
            expect_location(0, tombstone, &[9, 0, 0, 0]);

            section = section.L8(DW_LLE_default_location.0).uleb(4).L32(10);
            expect_location(0, u64::MAX, &[10, 0, 0, 0]);

            section = section.L8(DW_LLE_base_addressx.0).uleb(0);
            section = section.L8(DW_LLE_offset_pair.0).uleb(0x10100).uleb(0x10200);
            section = section.uleb(4).L32(11);
            expect_location(0x0301_0100, 0x0301_0200, &[11, 0, 0, 0]);

            section = section.L8(DW_LLE_startx_endx.0).uleb(1).uleb(2);
            section = section.uleb(4).L32(12);
            expect_location(0x0301_0300, 0x0301_0400, &[12, 0, 0, 0]);

            section = section.L8(DW_LLE_startx_length.0).uleb(3).uleb(0x100);
            section = section.uleb(4).L32(13);
            expect_location(0x0301_0500, 0x0301_0600, &[13, 0, 0, 0]);

            // Tombstone entries, all of which should be ignored.
            section = section.L8(DW_LLE_base_addressx.0).uleb(4);
            section = section.L8(DW_LLE_offset_pair.0).uleb(0x11100).uleb(0x11200);
            section = section.uleb(4).L32(20);

            section = section.L8(DW_LLE_base_address.0).word(size, tombstone);
            section = section.L8(DW_LLE_offset_pair.0).uleb(0x11300).uleb(0x11400);
            section = section.uleb(4).L32(21);

            section = section.L8(DW_LLE_startx_endx.0).uleb(4).uleb(5);
            section = section.uleb(4).L32(22);
            section = section.L8(DW_LLE_startx_length.0).uleb(4).uleb(0x100);
            section = section.uleb(4).L32(23);
            section = section
                .L8(DW_LLE_start_end.0)
                .word(size, tombstone)
                .word(size, 0x201_1500);
            section = section.uleb(4).L32(24);
            section = section
                .L8(DW_LLE_start_length.0)
                .word(size, tombstone)
                .uleb(0x100);
            section = section.uleb(4).L32(25);

            // Ignore some instances of 0 for tombstone.
            section = section.L8(DW_LLE_startx_endx.0).uleb(6).uleb(6);
            section = section.uleb(4).L32(30);
            section = section
                .L8(DW_LLE_start_end.0)
                .word(size, tombstone_0)
                .word(size, tombstone_0);
            section = section.uleb(4).L32(31);

            // Ignore empty ranges.
            section = section.L8(DW_LLE_base_address.0).word(size, 0);
            section = section.L8(DW_LLE_offset_pair.0).uleb(0).uleb(0);
            section = section.uleb(4).L32(41);
            section = section.L8(DW_LLE_base_address.0).word(size, 0x10000);
            section = section.L8(DW_LLE_offset_pair.0).uleb(0x1234).uleb(0x1234);
            section = section.uleb(4).L32(42);

            // A valid range after the tombstones.
            section = section
                .L8(DW_LLE_start_end.0)
                .word(size, 0x201_1600)
                .word(size, 0x201_1700);
            section = section.uleb(4).L32(100);
            expect_location(0x0201_1600, 0x0201_1700, &[100, 0, 0, 0]);

            section = section.L8(DW_LLE_end_of_list.0);
            section = section.mark(&end);
            // Some extra data.
            section = section.word(size, 0x1234_5678);
            length.set_const((&end - &start) as u64);

            let offset = LocationListsOffset((&first - &section.start()) as usize);
            let buf = section.get_contents().unwrap();
            let debug_loc = DebugLoc::new(&[], LittleEndian);
            let debug_loclists = DebugLocLists::new(&buf, LittleEndian);
            let loclists = LocationLists::new(debug_loc, debug_loclists);
            let mut locations = loclists
                .locations(offset, encoding, 0x0100_0000, debug_addr, debug_addr_base)
                .unwrap();

            for expected_location in expected_locations {
                let location = locations.next();
                assert_eq!(
                    location,
                    Ok(Some(expected_location)),
                    "read {:x?}, expect {:x?}",
                    location,
                    expected_location
                );
            }
            assert_eq!(locations.next(), Ok(None));
        }
    }

    #[test]
    fn test_location_list() {
        for size in [4, 8] {
            let base = u64::ones_sized(size);
            let tombstone = u64::ones_sized(size) - 1;
            let start = Label::new();
            let first = Label::new();
            let mut section = Section::with_endian(Endian::Little)
                // A location before the offset.
                .mark(&start)
                .word(size, 0x10000)
                .word(size, 0x10100)
                .L16(4)
                .L32(1)
                .mark(&first);

            let mut expected_locations = Vec::new();
            let mut expect_location = |begin, end, data| {
                expected_locations.push(LocationListEntry {
                    range: Range { begin, end },
                    data: Expression(EndianSlice::new(data, LittleEndian)),
                });
            };

            // A normal location.
            section = section.word(size, 0x10200).word(size, 0x10300);
            section = section.L16(4).L32(2);
            expect_location(0x0101_0200, 0x0101_0300, &[2, 0, 0, 0]);
            // A base address selection followed by a normal location.
            section = section.word(size, base).word(size, 0x0200_0000);
            section = section.word(size, 0x10400).word(size, 0x10500);
            section = section.L16(4).L32(3);
            expect_location(0x0201_0400, 0x0201_0500, &[3, 0, 0, 0]);
            // An empty location range followed by a normal location.
            section = section.word(size, 0x10600).word(size, 0x10600);
            section = section.L16(4).L32(4);
            section = section.word(size, 0x10800).word(size, 0x10900);
            section = section.L16(4).L32(5);
            expect_location(0x0201_0800, 0x0201_0900, &[5, 0, 0, 0]);
            // A location range that starts at 0.
            section = section.word(size, base).word(size, 0);
            section = section.word(size, 0).word(size, 1);
            section = section.L16(4).L32(6);
            expect_location(0, 1, &[6, 0, 0, 0]);
            // A location range that ends at -1.
            section = section.word(size, base).word(size, 0);
            section = section.word(size, 0).word(size, base);
            section = section.L16(4).L32(7);
            expect_location(0, base, &[7, 0, 0, 0]);
            // A normal location with tombstone.
            section = section.word(size, tombstone).word(size, tombstone);
            section = section.L16(4).L32(8);
            // A base address selection with tombstone followed by a normal location.
            section = section.word(size, base).word(size, tombstone);
            section = section.word(size, 0x10a00).word(size, 0x10b00);
            section = section.L16(4).L32(9);
            // A location list end.
            section = section.word(size, 0).word(size, 0);
            // Some extra data.
            section = section.word(size, 0x1234_5678);

            let buf = section.get_contents().unwrap();
            let debug_loc = DebugLoc::new(&buf, LittleEndian);
            let debug_loclists = DebugLocLists::new(&[], LittleEndian);
            let loclists = LocationLists::new(debug_loc, debug_loclists);
            let offset = LocationListsOffset((&first - &start) as usize);
            let debug_addr = &DebugAddr::from(EndianSlice::new(&[], LittleEndian));
            let debug_addr_base = DebugAddrBase(0);
            let encoding = Encoding {
                format: Format::Dwarf32,
                version: 4,
                address_size: size,
            };
            let mut locations = loclists
                .locations(offset, encoding, 0x0100_0000, debug_addr, debug_addr_base)
                .unwrap();

            for expected_location in expected_locations {
                let location = locations.next();
                assert_eq!(
                    location,
                    Ok(Some(expected_location)),
                    "read {:x?}, expect {:x?}",
                    location,
                    expected_location
                );
            }
            assert_eq!(locations.next(), Ok(None));

            // An offset at the end of buf.
            let mut locations = loclists
                .locations(
                    LocationListsOffset(buf.len()),
                    encoding,
                    0x0100_0000,
                    debug_addr,
                    debug_addr_base,
                )
                .unwrap();
            assert_eq!(locations.next(), Ok(None));
        }
    }

    #[test]
    fn test_locations_invalid() {
        #[rustfmt::skip]
        let section = Section::with_endian(Endian::Little)
            // An invalid location range.
            .L32(0x20000).L32(0x10000).L16(4).L32(1)
            // An invalid range after wrapping.
            .L32(0x20000).L32(0xff01_0000).L16(4).L32(2);

        let buf = section.get_contents().unwrap();
        let debug_loc = DebugLoc::new(&buf, LittleEndian);
        let debug_loclists = DebugLocLists::new(&[], LittleEndian);
        let loclists = LocationLists::new(debug_loc, debug_loclists);
        let debug_addr = &DebugAddr::from(EndianSlice::new(&[], LittleEndian));
        let debug_addr_base = DebugAddrBase(0);
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 4,
            address_size: 4,
        };

        // An invalid location range.
        let mut locations = loclists
            .locations(
                LocationListsOffset(0x0),
                encoding,
                0x0100_0000,
                debug_addr,
                debug_addr_base,
            )
            .unwrap();
        assert_eq!(locations.next(), Ok(None));

        // An invalid location range after wrapping.
        let mut locations = loclists
            .locations(
                LocationListsOffset(14),
                encoding,
                0x0100_0000,
                debug_addr,
                debug_addr_base,
            )
            .unwrap();
        assert_eq!(locations.next(), Ok(None));

        // An invalid offset.
        match loclists.locations(
            LocationListsOffset(buf.len() + 1),
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

            let debug_loc = DebugLoc::from(EndianSlice::new(&[], LittleEndian));
            let debug_loclists = DebugLocLists::from(EndianSlice::new(&section, LittleEndian));
            let locations = LocationLists::new(debug_loc, debug_loclists);

            let base = DebugLocListsBase((&first - &zero) as usize);
            assert_eq!(
                locations.get_offset(encoding, base, DebugLocListsIndex(0)),
                Ok(LocationListsOffset(base.0 + 1000))
            );
            assert_eq!(
                locations.get_offset(encoding, base, DebugLocListsIndex(19)),
                Ok(LocationListsOffset(base.0 + 1019))
            );
        }
    }

    #[test]
    fn test_loclists_gnu_v4_split_dwarf() {
        #[rustfmt::skip]
        let buf = [
            0x03, // DW_LLE_startx_length
            0x00, // ULEB encoded b7
            0x08, 0x00, 0x00, 0x00, // Fixed 4 byte length of 8
            0x03, 0x00, // Fixed two byte length of the location
            0x11, 0x00, // DW_OP_constu 0
            0x9f, // DW_OP_stack_value
            // Padding data
            //0x99, 0x99, 0x99, 0x99
        ];
        let data_buf = [0x11, 0x00, 0x9f];
        let expected_data = EndianSlice::new(&data_buf, LittleEndian);
        let debug_loc = DebugLoc::new(&buf, LittleEndian);
        let debug_loclists = DebugLocLists::new(&[], LittleEndian);
        let loclists = LocationLists::new(debug_loc, debug_loclists);
        let debug_addr =
            &DebugAddr::from(EndianSlice::new(&[0x01, 0x02, 0x03, 0x04], LittleEndian));
        let debug_addr_base = DebugAddrBase(0);
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 4,
            address_size: 4,
        };

        // An invalid location range.
        let mut locations = loclists
            .locations_dwo(
                LocationListsOffset(0x0),
                encoding,
                0,
                debug_addr,
                debug_addr_base,
            )
            .unwrap();
        assert_eq!(
            locations.next(),
            Ok(Some(LocationListEntry {
                range: Range {
                    begin: 0x0403_0201,
                    end: 0x0403_0209
                },
                data: Expression(expected_data),
            }))
        );
    }
}
