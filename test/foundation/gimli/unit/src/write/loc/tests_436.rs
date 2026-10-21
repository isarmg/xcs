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
        ConvertUnitContext, EndianVec, LineStringTable, RangeListTable, StringTable,
    };
    use crate::LittleEndian;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[test]
    fn test_loc_list() {
        let mut line_strings = LineStringTable::default();
        let mut strings = StringTable::default();
        let mut expression = Expression::new();
        expression.op_constu(0);

        for &version in &[2, 3, 4, 5] {
            for &address_size in &[4, 8] {
                for &format in &[Format::Dwarf32, Format::Dwarf64] {
                    let encoding = Encoding {
                        format,
                        version,
                        address_size,
                    };

                    let mut loc_list = LocationList(vec![
                        Location::StartLength {
                            begin: Address::Constant(6666),
                            length: 7777,
                            data: expression.clone(),
                        },
                        Location::StartEnd {
                            begin: Address::Constant(4444),
                            end: Address::Constant(5555),
                            data: expression.clone(),
                        },
                        Location::BaseAddress {
                            address: Address::Constant(1111),
                        },
                        Location::OffsetPair {
                            begin: 2222,
                            end: 3333,
                            data: expression.clone(),
                        },
                    ]);
                    if version >= 5 {
                        loc_list.0.push(Location::DefaultLocation {
                            data: expression.clone(),
                        });
                    }

                    let mut locations = LocationListTable::default();
                    let loc_list_id = locations.add(loc_list.clone());

                    let mut sections = Sections::new(EndianVec::new(LittleEndian));
                    let loc_list_offsets = locations.write(&mut sections, encoding, None).unwrap();
                    assert!(sections.debug_loc_refs.is_empty());
                    assert!(sections.debug_loclists_refs.is_empty());

                    let read_debug_loc =
                        read::DebugLoc::new(sections.debug_loc.slice(), LittleEndian);
                    let read_debug_loclists =
                        read::DebugLocLists::new(sections.debug_loclists.slice(), LittleEndian);
                    let read_loc = read::LocationLists::new(read_debug_loc, read_debug_loclists);
                    let offset = loc_list_offsets.get(loc_list_id);
                    let read_loc_list = read_loc.raw_locations(offset, encoding).unwrap();

                    let dwarf = read::Dwarf {
                        locations: read_loc,
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
                        ranges: &mut RangeListTable::default(),
                        locations: &mut locations,
                        convert_address: &|address| Some(Address::Constant(address)),
                        base_address: Address::Constant(0),
                        line_program_offset: None,
                        line_program_files: Vec::new(),
                        entry_ids: &HashMap::new(),
                    };
                    let convert_loc_list = LocationList::from(read_loc_list, &context).unwrap();

                    if version <= 4 {
                        loc_list.0[0] = Location::StartEnd {
                            begin: Address::Constant(6666),
                            end: Address::Constant(6666 + 7777),
                            data: expression.clone(),
                        };
                    }
                    assert_eq!(loc_list, convert_loc_list);
                }
            }
        }
    }
}
