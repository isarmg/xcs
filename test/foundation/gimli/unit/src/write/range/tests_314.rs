#[cfg(test)]
#[cfg(feature = "read")]
mod tests {
    use super::*;
    use crate::common::{
        DebugAbbrevOffset, DebugAddrBase, DebugInfoOffset, DebugLocListsBase, DebugRngListsBase,
        DebugStrOffsetsBase, Format,
    };
    use crate::read;
    use crate::write::{
        ConvertUnitContext, EndianVec, LineStringTable, LocationListTable, Range, RangeListTable,
        StringTable,
    };
    use crate::LittleEndian;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[test]
    fn test_range() {
        let mut line_strings = LineStringTable::default();
        let mut strings = StringTable::default();

        for &version in &[2, 3, 4, 5] {
            for &address_size in &[4, 8] {
                for &format in &[Format::Dwarf32, Format::Dwarf64] {
                    let encoding = Encoding {
                        format,
                        version,
                        address_size,
                    };

                    let mut range_list = RangeList(vec![
                        Range::StartLength {
                            begin: Address::Constant(6666),
                            length: 7777,
                        },
                        Range::StartEnd {
                            begin: Address::Constant(4444),
                            end: Address::Constant(5555),
                        },
                        Range::BaseAddress {
                            address: Address::Constant(1111),
                        },
                        Range::OffsetPair {
                            begin: 2222,
                            end: 3333,
                        },
                    ]);

                    let mut ranges = RangeListTable::default();
                    let range_list_id = ranges.add(range_list.clone());

                    let mut sections = Sections::new(EndianVec::new(LittleEndian));
                    let range_list_offsets = ranges.write(&mut sections, encoding).unwrap();

                    let read_debug_ranges =
                        read::DebugRanges::new(sections.debug_ranges.slice(), LittleEndian);
                    let read_debug_rnglists =
                        read::DebugRngLists::new(sections.debug_rnglists.slice(), LittleEndian);
                    let read_ranges = read::RangeLists::new(read_debug_ranges, read_debug_rnglists);
                    let offset = range_list_offsets.get(range_list_id);
                    let read_range_list = read_ranges.raw_ranges(offset, encoding).unwrap();

                    let dwarf = read::Dwarf {
                        ranges: read_ranges,
                        ..Default::default()
                    };
                    let unit = read::Unit {
                        header: read::UnitHeader::new(
                            encoding,
                            0,
                            read::UnitType::Compilation,
                            DebugAbbrevOffset(0),
                            DebugInfoOffset(0).into(),
                            read::EndianSlice::default(),
                        ),
                        abbreviations: Arc::new(read::Abbreviations::default()),
                        name: None,
                        comp_dir: None,
                        low_pc: 0,
                        str_offsets_base: DebugStrOffsetsBase(0),
                        addr_base: DebugAddrBase(0),
                        loclists_base: DebugLocListsBase(0),
                        rnglists_base: DebugRngListsBase(0),
                        line_program: None,
                        dwo_id: None,
                    };
                    let context = ConvertUnitContext {
                        dwarf: &dwarf,
                        unit: &unit,
                        line_strings: &mut line_strings,
                        strings: &mut strings,
                        ranges: &mut ranges,
                        locations: &mut LocationListTable::default(),
                        convert_address: &|address| Some(Address::Constant(address)),
                        base_address: Address::Constant(0),
                        line_program_offset: None,
                        line_program_files: Vec::new(),
                        entry_ids: &HashMap::new(),
                    };
                    let convert_range_list = RangeList::from(read_range_list, &context).unwrap();

                    if version <= 4 {
                        range_list.0[0] = Range::StartEnd {
                            begin: Address::Constant(6666),
                            end: Address::Constant(6666 + 7777),
                        };
                    }
                    assert_eq!(range_list, convert_range_list);
                }
            }
        }
    }
}
