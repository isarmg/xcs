#[cfg(test)]
#[cfg(feature = "read")]
mod tests {
    use super::*;
    use crate::common::LineEncoding;
    use crate::constants;
    use crate::read;
    use crate::write::{
        Dwarf, DwarfUnit, EndianVec, LineString, Location, LocationList, Range, RangeList,
    };
    use crate::LittleEndian;
    use std::mem;

    #[test]
    fn test_unit_table() {
        let mut dwarf = Dwarf::new();
        let unit_id1 = dwarf.units.add(Unit::new(
            Encoding {
                version: 4,
                address_size: 8,
                format: Format::Dwarf32,
            },
            LineProgram::none(),
        ));
        let unit2 = dwarf.units.add(Unit::new(
            Encoding {
                version: 2,
                address_size: 4,
                format: Format::Dwarf64,
            },
            LineProgram::none(),
        ));
        let unit3 = dwarf.units.add(Unit::new(
            Encoding {
                version: 5,
                address_size: 4,
                format: Format::Dwarf32,
            },
            LineProgram::none(),
        ));
        assert_eq!(dwarf.units.count(), 3);
        {
            let unit1 = dwarf.units.get_mut(unit_id1);
            assert_eq!(unit1.version(), 4);
            assert_eq!(unit1.address_size(), 8);
            assert_eq!(unit1.format(), Format::Dwarf32);
            assert_eq!(unit1.count(), 1);

            let root_id = unit1.root();
            assert_eq!(root_id, UnitEntryId::new(unit1.base_id, 0));
            {
                let root = unit1.get_mut(root_id);
                assert_eq!(root.id(), root_id);
                assert!(root.parent().is_none());
                assert_eq!(root.tag(), constants::DW_TAG_compile_unit);

                // Test get/get_mut
                assert!(root.get(constants::DW_AT_producer).is_none());
                assert!(root.get_mut(constants::DW_AT_producer).is_none());
                let mut producer = AttributeValue::String(b"root"[..].into());
                root.set(constants::DW_AT_producer, producer.clone());
                assert_eq!(root.get(constants::DW_AT_producer), Some(&producer));
                assert_eq!(root.get_mut(constants::DW_AT_producer), Some(&mut producer));

                // Test attrs
                let mut attrs = root.attrs();
                let attr = attrs.next().unwrap();
                assert_eq!(attr.name(), constants::DW_AT_producer);
                assert_eq!(attr.get(), &producer);
                assert!(attrs.next().is_none());
            }

            let child1 = unit1.add(root_id, constants::DW_TAG_subprogram);
            assert_eq!(child1, UnitEntryId::new(unit1.base_id, 1));
            {
                let child1 = unit1.get_mut(child1);
                assert_eq!(child1.parent(), Some(root_id));

                let tmp = AttributeValue::String(b"tmp"[..].into());
                child1.set(constants::DW_AT_name, tmp.clone());
                assert_eq!(child1.get(constants::DW_AT_name), Some(&tmp));

                // Test attrs_mut
                let name = AttributeValue::StringRef(dwarf.strings.add(&b"child1"[..]));
                {
                    let attr = child1.attrs_mut().next().unwrap();
                    assert_eq!(attr.name(), constants::DW_AT_name);
                    attr.set(name.clone());
                }
                assert_eq!(child1.get(constants::DW_AT_name), Some(&name));
            }

            let child2 = unit1.add(root_id, constants::DW_TAG_subprogram);
            assert_eq!(child2, UnitEntryId::new(unit1.base_id, 2));
            {
                let child2 = unit1.get_mut(child2);
                assert_eq!(child2.parent(), Some(root_id));

                let tmp = AttributeValue::String(b"tmp"[..].into());
                child2.set(constants::DW_AT_name, tmp.clone());
                assert_eq!(child2.get(constants::DW_AT_name), Some(&tmp));

                // Test replace
                let name = AttributeValue::StringRef(dwarf.strings.add(&b"child2"[..]));
                child2.set(constants::DW_AT_name, name.clone());
                assert_eq!(child2.get(constants::DW_AT_name), Some(&name));
            }

            {
                let root = unit1.get(root_id);
                assert_eq!(
                    root.children().cloned().collect::<Vec<_>>(),
                    vec![child1, child2]
                );
            }
        }
        {
            let unit2 = dwarf.units.get(unit2);
            assert_eq!(unit2.version(), 2);
            assert_eq!(unit2.address_size(), 4);
            assert_eq!(unit2.format(), Format::Dwarf64);
            assert_eq!(unit2.count(), 1);

            let root = unit2.root();
            assert_eq!(root, UnitEntryId::new(unit2.base_id, 0));
            let root = unit2.get(root);
            assert_eq!(root.id(), UnitEntryId::new(unit2.base_id, 0));
            assert!(root.parent().is_none());
            assert_eq!(root.tag(), constants::DW_TAG_compile_unit);
        }

        let mut sections = Sections::new(EndianVec::new(LittleEndian));
        dwarf.write(&mut sections).unwrap();

        println!("{:?}", sections.debug_str);
        println!("{:?}", sections.debug_info);
        println!("{:?}", sections.debug_abbrev);

        let read_dwarf = sections.read(LittleEndian);
        let mut read_units = read_dwarf.units();

        {
            let read_unit1 = read_units.next().unwrap().unwrap();
            let unit1 = dwarf.units.get(unit_id1);
            assert_eq!(unit1.version(), read_unit1.version());
            assert_eq!(unit1.address_size(), read_unit1.address_size());
            assert_eq!(unit1.format(), read_unit1.format());

            let read_unit1 = read_dwarf.unit(read_unit1).unwrap();
            let mut read_entries = read_unit1.entries();

            let root = unit1.get(unit1.root());
            {
                let (depth, read_root) = read_entries.next_dfs().unwrap().unwrap();
                assert_eq!(depth, 0);
                assert_eq!(root.tag(), read_root.tag());
                assert!(read_root.has_children());

                let producer = match root.get(constants::DW_AT_producer).unwrap() {
                    AttributeValue::String(ref producer) => &**producer,
                    otherwise => panic!("unexpected {:?}", otherwise),
                };
                assert_eq!(producer, b"root");
                let read_producer = read_root
                    .attr_value(constants::DW_AT_producer)
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    read_dwarf
                        .attr_string(&read_unit1, read_producer)
                        .unwrap()
                        .slice(),
                    producer
                );
            }

            let mut children = root.children().cloned();

            {
                let child = children.next().unwrap();
                assert_eq!(child, UnitEntryId::new(unit1.base_id, 1));
                let child = unit1.get(child);
                let (depth, read_child) = read_entries.next_dfs().unwrap().unwrap();
                assert_eq!(depth, 1);
                assert_eq!(child.tag(), read_child.tag());
                assert!(!read_child.has_children());

                let name = match child.get(constants::DW_AT_name).unwrap() {
                    AttributeValue::StringRef(name) => *name,
                    otherwise => panic!("unexpected {:?}", otherwise),
                };
                let name = dwarf.strings.get(name);
                assert_eq!(name, b"child1");
                let read_name = read_child
                    .attr_value(constants::DW_AT_name)
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    read_dwarf
                        .attr_string(&read_unit1, read_name)
                        .unwrap()
                        .slice(),
                    name
                );
            }

            {
                let child = children.next().unwrap();
                assert_eq!(child, UnitEntryId::new(unit1.base_id, 2));
                let child = unit1.get(child);
                let (depth, read_child) = read_entries.next_dfs().unwrap().unwrap();
                assert_eq!(depth, 0);
                assert_eq!(child.tag(), read_child.tag());
                assert!(!read_child.has_children());

                let name = match child.get(constants::DW_AT_name).unwrap() {
                    AttributeValue::StringRef(name) => *name,
                    otherwise => panic!("unexpected {:?}", otherwise),
                };
                let name = dwarf.strings.get(name);
                assert_eq!(name, b"child2");
                let read_name = read_child
                    .attr_value(constants::DW_AT_name)
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    read_dwarf
                        .attr_string(&read_unit1, read_name)
                        .unwrap()
                        .slice(),
                    name
                );
            }

            assert!(read_entries.next_dfs().unwrap().is_none());
        }

        {
            let read_unit2 = read_units.next().unwrap().unwrap();
            let unit2 = dwarf.units.get(unit2);
            assert_eq!(unit2.version(), read_unit2.version());
            assert_eq!(unit2.address_size(), read_unit2.address_size());
            assert_eq!(unit2.format(), read_unit2.format());

            let abbrevs = read_dwarf.abbreviations(&read_unit2).unwrap();
            let mut read_entries = read_unit2.entries(&abbrevs);

            {
                let root = unit2.get(unit2.root());
                let (depth, read_root) = read_entries.next_dfs().unwrap().unwrap();
                assert_eq!(depth, 0);
                assert_eq!(root.tag(), read_root.tag());
                assert!(!read_root.has_children());
            }

            assert!(read_entries.next_dfs().unwrap().is_none());
        }

        {
            let read_unit3 = read_units.next().unwrap().unwrap();
            let unit3 = dwarf.units.get(unit3);
            assert_eq!(unit3.version(), read_unit3.version());
            assert_eq!(unit3.address_size(), read_unit3.address_size());
            assert_eq!(unit3.format(), read_unit3.format());

            let abbrevs = read_dwarf.abbreviations(&read_unit3).unwrap();
            let mut read_entries = read_unit3.entries(&abbrevs);

            {
                let root = unit3.get(unit3.root());
                let (depth, read_root) = read_entries.next_dfs().unwrap().unwrap();
                assert_eq!(depth, 0);
                assert_eq!(root.tag(), read_root.tag());
                assert!(!read_root.has_children());
            }

            assert!(read_entries.next_dfs().unwrap().is_none());
        }

        assert!(read_units.next().unwrap().is_none());

        let convert_dwarf =
            Dwarf::from(&read_dwarf, &|address| Some(Address::Constant(address))).unwrap();
        assert_eq!(convert_dwarf.units.count(), dwarf.units.count());

        for i in 0..convert_dwarf.units.count() {
            let unit_id = dwarf.units.id(i);
            let unit = dwarf.units.get(unit_id);
            let convert_unit_id = convert_dwarf.units.id(i);
            let convert_unit = convert_dwarf.units.get(convert_unit_id);
            assert_eq!(convert_unit.version(), unit.version());
            assert_eq!(convert_unit.address_size(), unit.address_size());
            assert_eq!(convert_unit.format(), unit.format());
            assert_eq!(convert_unit.count(), unit.count());

            let root = unit.get(unit.root());
            let convert_root = convert_unit.get(convert_unit.root());
            assert_eq!(convert_root.tag(), root.tag());
            for (convert_attr, attr) in convert_root.attrs().zip(root.attrs()) {
                assert_eq!(convert_attr, attr);
            }
        }
    }

    #[test]
    fn test_attribute_value() {
        let string_data = "string data";
        let line_string_data = "line string data";

        let data = vec![1, 2, 3, 4];
        let read_data = read::EndianSlice::new(&[1, 2, 3, 4], LittleEndian);

        let mut expression = Expression::new();
        expression.op_constu(57);
        let read_expression = read::Expression(read::EndianSlice::new(
            &[constants::DW_OP_constu.0, 57],
            LittleEndian,
        ));

        let range = RangeList(vec![Range::StartEnd {
            begin: Address::Constant(0x1234),
            end: Address::Constant(0x2345),
        }]);

        let location = LocationList(vec![Location::StartEnd {
            begin: Address::Constant(0x1234),
            end: Address::Constant(0x2345),
            data: expression.clone(),
        }]);

        for &version in &[2, 3, 4, 5] {
            for &address_size in &[4, 8] {
                for &format in &[Format::Dwarf32, Format::Dwarf64] {
                    let encoding = Encoding {
                        format,
                        version,
                        address_size,
                    };

                    let mut dwarf = Dwarf::new();
                    let unit = dwarf.units.add(Unit::new(encoding, LineProgram::none()));
                    let unit = dwarf.units.get_mut(unit);
                    let loc_id = unit.locations.add(location.clone());
                    let range_id = unit.ranges.add(range.clone());
                    // Create a string with a non-zero id/offset.
                    dwarf.strings.add("dummy string");
                    let string_id = dwarf.strings.add(string_data);
                    dwarf.line_strings.add("dummy line string");
                    let line_string_id = dwarf.line_strings.add(line_string_data);

                    let attributes = &[
                        (
                            constants::DW_AT_name,
                            AttributeValue::Address(Address::Constant(0x1234)),
                            read::AttributeValue::Addr(0x1234),
                        ),
                        (
                            constants::DW_AT_name,
                            AttributeValue::Block(data.clone()),
                            read::AttributeValue::Block(read_data),
                        ),
                        (
                            constants::DW_AT_name,
                            AttributeValue::Data1(0x12),
                            read::AttributeValue::Data1(0x12),
                        ),
                        (
                            constants::DW_AT_name,
                            AttributeValue::Data2(0x1234),
                            read::AttributeValue::Data2(0x1234),
                        ),
                        (
                            constants::DW_AT_name,
                            AttributeValue::Data4(0x1234),
                            read::AttributeValue::Data4(0x1234),
                        ),
                        (
                            constants::DW_AT_name,
                            AttributeValue::Data8(0x1234),
                            read::AttributeValue::Data8(0x1234),
                        ),
                        (
                            constants::DW_AT_name,
                            AttributeValue::Sdata(0x1234),
                            read::AttributeValue::Sdata(0x1234),
                        ),
                        (
                            constants::DW_AT_name,
                            AttributeValue::Udata(0x1234),
                            read::AttributeValue::Udata(0x1234),
                        ),
                        (
                            constants::DW_AT_name,
                            AttributeValue::Exprloc(expression.clone()),
                            read::AttributeValue::Exprloc(read_expression),
                        ),
                        (
                            constants::DW_AT_name,
                            AttributeValue::Flag(false),
                            read::AttributeValue::Flag(false),
                        ),
                        /*
                        (
                            constants::DW_AT_name,
                            AttributeValue::FlagPresent,
                            read::AttributeValue::Flag(true),
                        ),
                        */
                        (
                            constants::DW_AT_name,
                            AttributeValue::DebugInfoRefSup(DebugInfoOffset(0x1234)),
                            read::AttributeValue::DebugInfoRefSup(DebugInfoOffset(0x1234)),
                        ),
                        (
                            constants::DW_AT_macro_info,
                            AttributeValue::DebugMacinfoRef(DebugMacinfoOffset(0x1234)),
                            read::AttributeValue::SecOffset(0x1234),
                        ),
                        (
                            constants::DW_AT_macros,
                            AttributeValue::DebugMacroRef(DebugMacroOffset(0x1234)),
                            read::AttributeValue::SecOffset(0x1234),
                        ),
                        (
                            constants::DW_AT_name,
                            AttributeValue::DebugTypesRef(DebugTypeSignature(0x1234)),
                            read::AttributeValue::DebugTypesRef(DebugTypeSignature(0x1234)),
                        ),
                        (
                            constants::DW_AT_name,
                            AttributeValue::DebugStrRefSup(DebugStrOffset(0x1234)),
                            read::AttributeValue::DebugStrRefSup(DebugStrOffset(0x1234)),
                        ),
                        (
                            constants::DW_AT_name,
                            AttributeValue::String(data.clone()),
                            read::AttributeValue::String(read_data),
                        ),
                        (
                            constants::DW_AT_encoding,
                            AttributeValue::Encoding(constants::DwAte(0x12)),
                            read::AttributeValue::Udata(0x12),
                        ),
                        (
                            constants::DW_AT_decimal_sign,
                            AttributeValue::DecimalSign(constants::DwDs(0x12)),
                            read::AttributeValue::Udata(0x12),
                        ),
                        (
                            constants::DW_AT_endianity,
                            AttributeValue::Endianity(constants::DwEnd(0x12)),
                            read::AttributeValue::Udata(0x12),
                        ),
                        (
                            constants::DW_AT_accessibility,
                            AttributeValue::Accessibility(constants::DwAccess(0x12)),
                            read::AttributeValue::Udata(0x12),
                        ),
                        (
                            constants::DW_AT_visibility,
                            AttributeValue::Visibility(constants::DwVis(0x12)),
                            read::AttributeValue::Udata(0x12),
                        ),
                        (
                            constants::DW_AT_virtuality,
                            AttributeValue::Virtuality(constants::DwVirtuality(0x12)),
                            read::AttributeValue::Udata(0x12),
                        ),
                        (
                            constants::DW_AT_language,
                            AttributeValue::Language(constants::DwLang(0x12)),
                            read::AttributeValue::Udata(0x12),
                        ),
                        (
                            constants::DW_AT_address_class,
                            AttributeValue::AddressClass(constants::DwAddr(0x12)),
                            read::AttributeValue::Udata(0x12),
                        ),
                        (
                            constants::DW_AT_identifier_case,
                            AttributeValue::IdentifierCase(constants::DwId(0x12)),
                            read::AttributeValue::Udata(0x12),
                        ),
                        (
                            constants::DW_AT_calling_convention,
                            AttributeValue::CallingConvention(constants::DwCc(0x12)),
                            read::AttributeValue::Udata(0x12),
                        ),
                        (
                            constants::DW_AT_ordering,
                            AttributeValue::Ordering(constants::DwOrd(0x12)),
                            read::AttributeValue::Udata(0x12),
                        ),
                        (
                            constants::DW_AT_inline,
                            AttributeValue::Inline(constants::DwInl(0x12)),
                            read::AttributeValue::Udata(0x12),
                        ),
                    ];

                    let mut add_attribute = |name, value| {
                        let entry_id = unit.add(unit.root(), constants::DW_TAG_subprogram);
                        let entry = unit.get_mut(entry_id);
                        entry.set(name, value);
                    };
                    for (name, value, _) in attributes {
                        add_attribute(*name, value.clone());
                    }
                    add_attribute(
                        constants::DW_AT_location,
                        AttributeValue::LocationListRef(loc_id),
                    );
                    add_attribute(
                        constants::DW_AT_ranges,
                        AttributeValue::RangeListRef(range_id),
                    );
                    add_attribute(constants::DW_AT_name, AttributeValue::StringRef(string_id));
                    add_attribute(
                        constants::DW_AT_name,
                        AttributeValue::LineStringRef(line_string_id),
                    );

                    let mut sections = Sections::new(EndianVec::new(LittleEndian));
                    dwarf.write(&mut sections).unwrap();

                    let read_dwarf = sections.read(LittleEndian);
                    let mut read_units = read_dwarf.units();
                    let read_unit = read_units.next().unwrap().unwrap();
                    let read_unit = read_dwarf.unit(read_unit).unwrap();
                    let read_unit = read_unit.unit_ref(&read_dwarf);
                    let mut read_entries = read_unit.entries();
                    let (_, _root) = read_entries.next_dfs().unwrap().unwrap();

                    let mut get_attribute = |name| {
                        let (_, entry) = read_entries.next_dfs().unwrap().unwrap();
                        entry.attr(name).unwrap().unwrap()
                    };
                    for (name, _, expect_value) in attributes {
                        let read_value = &get_attribute(*name).raw_value();
                        // read::AttributeValue is invariant in the lifetime of R.
                        // The lifetimes here are all okay, so transmute it.
                        let read_value = unsafe {
                            mem::transmute::<
                                &read::AttributeValue<read::EndianSlice<'_, LittleEndian>>,
                                &read::AttributeValue<read::EndianSlice<'_, LittleEndian>>,
                            >(read_value)
                        };
                        assert_eq!(read_value, expect_value);
                    }

                    let read_attr = get_attribute(constants::DW_AT_location).value();
                    let read::AttributeValue::LocationListsRef(read_loc_offset) = read_attr else {
                        panic!("unexpected {:?}", read_attr);
                    };
                    let mut read_locations = read_unit.locations(read_loc_offset).unwrap();
                    let read_location = read_locations.next().unwrap().unwrap();
                    assert_eq!(read_location.range.begin, 0x1234);
                    assert_eq!(read_location.range.end, 0x2345);
                    assert_eq!(read_location.data, read_expression);

                    let read_attr = get_attribute(constants::DW_AT_ranges).value();
                    let read::AttributeValue::RangeListsRef(read_range_offset) = read_attr else {
                        panic!("unexpected {:?}", read_attr);
                    };
                    let read_range_offset = read_unit.ranges_offset_from_raw(read_range_offset);
                    let mut read_ranges = read_unit.ranges(read_range_offset).unwrap();
                    let read_range = read_ranges.next().unwrap().unwrap();
                    assert_eq!(read_range.begin, 0x1234);
                    assert_eq!(read_range.end, 0x2345);

                    let read_string = get_attribute(constants::DW_AT_name).raw_value();
                    let read::AttributeValue::DebugStrRef(read_string_offset) = read_string else {
                        panic!("unexpected {:?}", read_string);
                    };
                    assert_eq!(
                        read_dwarf.string(read_string_offset).unwrap().slice(),
                        string_data.as_bytes()
                    );

                    let read_line_string = get_attribute(constants::DW_AT_name).raw_value();
                    let read::AttributeValue::DebugLineStrRef(read_line_string_offset) =
                        read_line_string
                    else {
                        panic!("unexpected {:?}", read_line_string);
                    };
                    assert_eq!(
                        read_dwarf
                            .line_string(read_line_string_offset)
                            .unwrap()
                            .slice(),
                        line_string_data.as_bytes()
                    );

                    let convert_dwarf =
                        Dwarf::from(&read_dwarf, &|address| Some(Address::Constant(address)))
                            .unwrap();
                    let convert_unit = convert_dwarf.units.get(convert_dwarf.units.id(0));
                    let convert_root = convert_unit.get(convert_unit.root());
                    let mut convert_entries = convert_root.children();

                    let mut get_convert_attr = |name| {
                        let convert_entry = convert_unit.get(*convert_entries.next().unwrap());
                        convert_entry.get(name).unwrap()
                    };
                    for (name, attr, _) in attributes {
                        let convert_attr = get_convert_attr(*name);
                        assert_eq!(convert_attr, attr);
                    }

                    let convert_attr = get_convert_attr(constants::DW_AT_location);
                    let AttributeValue::LocationListRef(convert_loc_id) = convert_attr else {
                        panic!("unexpected {:?}", convert_attr);
                    };
                    let convert_location = convert_unit.locations.get(*convert_loc_id);
                    assert_eq!(*convert_location, location);

                    let convert_attr = get_convert_attr(constants::DW_AT_ranges);
                    let AttributeValue::RangeListRef(convert_range_id) = convert_attr else {
                        panic!("unexpected {:?}", convert_attr);
                    };
                    let convert_range = convert_unit.ranges.get(*convert_range_id);
                    assert_eq!(*convert_range, range);

                    let convert_attr = get_convert_attr(constants::DW_AT_name);
                    let AttributeValue::StringRef(convert_string_id) = convert_attr else {
                        panic!("unexpected {:?}", convert_attr);
                    };
                    let convert_string = convert_dwarf.strings.get(*convert_string_id);
                    assert_eq!(convert_string, string_data.as_bytes());

                    let convert_attr = get_convert_attr(constants::DW_AT_name);
                    let AttributeValue::LineStringRef(convert_line_string_id) = convert_attr else {
                        panic!("unexpected {:?}", convert_attr);
                    };
                    let convert_line_string =
                        convert_dwarf.line_strings.get(*convert_line_string_id);
                    assert_eq!(convert_line_string, line_string_data.as_bytes());
                }
            }
        }
    }

    #[test]
    fn test_unit_ref() {
        let mut dwarf = Dwarf::new();
        let unit_id1 = dwarf.units.add(Unit::new(
            Encoding {
                version: 4,
                address_size: 8,
                format: Format::Dwarf32,
            },
            LineProgram::none(),
        ));
        assert_eq!(unit_id1, dwarf.units.id(0));
        let unit_id2 = dwarf.units.add(Unit::new(
            Encoding {
                version: 2,
                address_size: 4,
                format: Format::Dwarf64,
            },
            LineProgram::none(),
        ));
        assert_eq!(unit_id2, dwarf.units.id(1));
        let unit1_child1 = UnitEntryId::new(dwarf.units.get(unit_id1).base_id, 1);
        let unit1_child2 = UnitEntryId::new(dwarf.units.get(unit_id1).base_id, 2);
        let unit2_child1 = UnitEntryId::new(dwarf.units.get(unit_id2).base_id, 1);
        let unit2_child2 = UnitEntryId::new(dwarf.units.get(unit_id2).base_id, 2);
        {
            let unit1 = dwarf.units.get_mut(unit_id1);
            let root = unit1.root();
            let child_id1 = unit1.add(root, constants::DW_TAG_subprogram);
            assert_eq!(child_id1, unit1_child1);
            let child_id2 = unit1.add(root, constants::DW_TAG_subprogram);
            assert_eq!(child_id2, unit1_child2);
            {
                let child1 = unit1.get_mut(child_id1);
                child1.set(constants::DW_AT_type, AttributeValue::UnitRef(child_id2));
            }
            {
                let child2 = unit1.get_mut(child_id2);
                child2.set(
                    constants::DW_AT_type,
                    AttributeValue::DebugInfoRef(Reference::Entry(unit_id2, unit2_child1)),
                );
            }
        }
        {
            let unit2 = dwarf.units.get_mut(unit_id2);
            let root = unit2.root();
            let child_id1 = unit2.add(root, constants::DW_TAG_subprogram);
            assert_eq!(child_id1, unit2_child1);
            let child_id2 = unit2.add(root, constants::DW_TAG_subprogram);
            assert_eq!(child_id2, unit2_child2);
            {
                let child1 = unit2.get_mut(child_id1);
                child1.set(constants::DW_AT_type, AttributeValue::UnitRef(child_id2));
            }
            {
                let child2 = unit2.get_mut(child_id2);
                child2.set(
                    constants::DW_AT_type,
                    AttributeValue::DebugInfoRef(Reference::Entry(unit_id1, unit1_child1)),
                );
            }
        }

        let mut sections = Sections::new(EndianVec::new(LittleEndian));
        dwarf.write(&mut sections).unwrap();

        println!("{:?}", sections.debug_info);
        println!("{:?}", sections.debug_abbrev);

        let read_dwarf = sections.read(LittleEndian);
        let mut read_units = read_dwarf.units();

        let read_unit = read_units.next().unwrap().unwrap();
        let abbrevs = read_dwarf.abbreviations(&read_unit).unwrap();
        let mut read_entries = read_unit.entries(&abbrevs);
        let (_, _root) = read_entries.next_dfs().unwrap().unwrap();
        let (_, entry) = read_entries.next_dfs().unwrap().unwrap();
        let read_unit1_child1_attr = entry.attr_value(constants::DW_AT_type).unwrap();
        let read_unit1_child1_section_offset =
            entry.offset().to_debug_info_offset(&read_unit).unwrap();
        let (_, entry) = read_entries.next_dfs().unwrap().unwrap();
        let read_unit1_child2_attr = entry.attr_value(constants::DW_AT_type).unwrap();
        let read_unit1_child2_offset = entry.offset();

        let read_unit = read_units.next().unwrap().unwrap();
        let abbrevs = read_dwarf.abbreviations(&read_unit).unwrap();
        let mut read_entries = read_unit.entries(&abbrevs);
        let (_, _root) = read_entries.next_dfs().unwrap().unwrap();
        let (_, entry) = read_entries.next_dfs().unwrap().unwrap();
        let read_unit2_child1_attr = entry.attr_value(constants::DW_AT_type).unwrap();
        let read_unit2_child1_section_offset =
            entry.offset().to_debug_info_offset(&read_unit).unwrap();
        let (_, entry) = read_entries.next_dfs().unwrap().unwrap();
        let read_unit2_child2_attr = entry.attr_value(constants::DW_AT_type).unwrap();
        let read_unit2_child2_offset = entry.offset();

        assert_eq!(
            read_unit1_child1_attr,
            Some(read::AttributeValue::UnitRef(read_unit1_child2_offset))
        );
        assert_eq!(
            read_unit1_child2_attr,
            Some(read::AttributeValue::DebugInfoRef(
                read_unit2_child1_section_offset
            ))
        );
        assert_eq!(
            read_unit2_child1_attr,
            Some(read::AttributeValue::UnitRef(read_unit2_child2_offset))
        );
        assert_eq!(
            read_unit2_child2_attr,
            Some(read::AttributeValue::DebugInfoRef(
                read_unit1_child1_section_offset
            ))
        );

        let convert_dwarf =
            Dwarf::from(&read_dwarf, &|address| Some(Address::Constant(address))).unwrap();
        let convert_units = &convert_dwarf.units;
        assert_eq!(convert_units.count(), dwarf.units.count());

        for i in 0..convert_units.count() {
            let unit = dwarf.units.get(dwarf.units.id(i));
            let convert_unit = convert_units.get(convert_units.id(i));
            assert_eq!(convert_unit.version(), unit.version());
            assert_eq!(convert_unit.address_size(), unit.address_size());
            assert_eq!(convert_unit.format(), unit.format());
            assert_eq!(convert_unit.count(), unit.count());

            let root = unit.get(unit.root());
            let convert_root = convert_unit.get(convert_unit.root());
            assert_eq!(convert_root.tag(), root.tag());
            for (convert_attr, attr) in convert_root.attrs().zip(root.attrs()) {
                assert_eq!(convert_attr, attr);
            }

            let child1 = unit.get(UnitEntryId::new(unit.base_id, 1));
            let convert_child1 = convert_unit.get(UnitEntryId::new(convert_unit.base_id, 1));
            assert_eq!(convert_child1.tag(), child1.tag());
            for (convert_attr, attr) in convert_child1.attrs().zip(child1.attrs()) {
                assert_eq!(convert_attr.name, attr.name);
                match (convert_attr.value.clone(), attr.value.clone()) {
                    (
                        AttributeValue::DebugInfoRef(Reference::Entry(convert_unit, convert_entry)),
                        AttributeValue::DebugInfoRef(Reference::Entry(unit, entry)),
                    ) => {
                        assert_eq!(convert_unit.index, unit.index);
                        assert_eq!(convert_entry.index, entry.index);
                    }
                    (AttributeValue::UnitRef(convert_id), AttributeValue::UnitRef(id)) => {
                        assert_eq!(convert_id.index, id.index);
                    }
                    (convert_value, value) => assert_eq!(convert_value, value),
                }
            }

            let child2 = unit.get(UnitEntryId::new(unit.base_id, 2));
            let convert_child2 = convert_unit.get(UnitEntryId::new(convert_unit.base_id, 2));
            assert_eq!(convert_child2.tag(), child2.tag());
            for (convert_attr, attr) in convert_child2.attrs().zip(child2.attrs()) {
                assert_eq!(convert_attr.name, attr.name);
                match (convert_attr.value.clone(), attr.value.clone()) {
                    (
                        AttributeValue::DebugInfoRef(Reference::Entry(convert_unit, convert_entry)),
                        AttributeValue::DebugInfoRef(Reference::Entry(unit, entry)),
                    ) => {
                        assert_eq!(convert_unit.index, unit.index);
                        assert_eq!(convert_entry.index, entry.index);
                    }
                    (AttributeValue::UnitRef(convert_id), AttributeValue::UnitRef(id)) => {
                        assert_eq!(convert_id.index, id.index);
                    }
                    (convert_value, value) => assert_eq!(convert_value, value),
                }
            }
        }
    }

    #[test]
    fn test_sibling() {
        fn add_child(
            unit: &mut Unit,
            parent: UnitEntryId,
            tag: constants::DwTag,
            name: &str,
        ) -> UnitEntryId {
            let id = unit.add(parent, tag);
            let child = unit.get_mut(id);
            child.set(constants::DW_AT_name, AttributeValue::String(name.into()));
            child.set_sibling(true);
            id
        }

        fn add_children(unit: &mut Unit) {
            let root = unit.root();
            let child1 = add_child(unit, root, constants::DW_TAG_subprogram, "child1");
            add_child(unit, child1, constants::DW_TAG_variable, "grandchild1");
            add_child(unit, root, constants::DW_TAG_subprogram, "child2");
            add_child(unit, root, constants::DW_TAG_subprogram, "child3");
        }

        fn next_child<R: read::Reader<Offset = usize>>(
            entries: &mut read::EntriesCursor<'_, '_, R>,
        ) -> (read::UnitOffset, Option<read::UnitOffset>) {
            let (_, entry) = entries.next_dfs().unwrap().unwrap();
            let offset = entry.offset();
            let sibling =
                entry
                    .attr_value(constants::DW_AT_sibling)
                    .unwrap()
                    .map(|attr| match attr {
                        read::AttributeValue::UnitRef(offset) => offset,
                        _ => panic!("bad sibling value"),
                    });
            (offset, sibling)
        }

        fn check_sibling<R: read::Reader<Offset = usize>>(
            unit: read::UnitHeader<R>,
            dwarf: &read::Dwarf<R>,
        ) {
            let unit = dwarf.unit(unit).unwrap();
            let mut entries = unit.entries();
            // root
            entries.next_dfs().unwrap().unwrap();
            // child1
            let (_, sibling1) = next_child(&mut entries);
            // grandchild1
            entries.next_dfs().unwrap().unwrap();
            // child2
            let (offset2, sibling2) = next_child(&mut entries);
            // child3
            let (_, _) = next_child(&mut entries);
            assert_eq!(sibling1, Some(offset2));
            assert_eq!(sibling2, None);
        }

        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 4,
            address_size: 8,
        };
        let mut dwarf = Dwarf::new();
        let unit_id1 = dwarf.units.add(Unit::new(encoding, LineProgram::none()));
        add_children(dwarf.units.get_mut(unit_id1));
        let unit_id2 = dwarf.units.add(Unit::new(encoding, LineProgram::none()));
        add_children(dwarf.units.get_mut(unit_id2));

        let mut sections = Sections::new(EndianVec::new(LittleEndian));
        dwarf.write(&mut sections).unwrap();

        println!("{:?}", sections.debug_info);
        println!("{:?}", sections.debug_abbrev);

        let read_dwarf = sections.read(LittleEndian);
        let mut read_units = read_dwarf.units();
        check_sibling(read_units.next().unwrap().unwrap(), &read_dwarf);
        check_sibling(read_units.next().unwrap().unwrap(), &read_dwarf);
    }

    #[test]
    fn test_line_ref() {
        let dir_bytes = b"dir";
        let file_bytes1 = b"file1";
        let file_bytes2 = b"file2";
        let file_string1 = LineString::String(file_bytes1.to_vec());
        let file_string2 = LineString::String(file_bytes2.to_vec());

        for &version in &[2, 3, 4, 5] {
            for &address_size in &[4, 8] {
                for &format in &[Format::Dwarf32, Format::Dwarf64] {
                    let encoding = Encoding {
                        format,
                        version,
                        address_size,
                    };

                    // The line program we'll be referencing.
                    let mut line_program = LineProgram::new(
                        encoding,
                        LineEncoding::default(),
                        LineString::String(dir_bytes.to_vec()),
                        None,
                        file_string1.clone(),
                        None,
                    );
                    let dir = line_program.default_directory();
                    // For version >= 5, this will reuse the existing file at index 0.
                    let file1 = line_program.add_file(file_string1.clone(), dir, None);
                    let file2 = line_program.add_file(file_string2.clone(), dir, None);

                    let mut unit = Unit::new(encoding, line_program);
                    let root = unit.get_mut(unit.root());
                    root.set(
                        constants::DW_AT_name,
                        AttributeValue::String(file_bytes1.to_vec()),
                    );
                    root.set(
                        constants::DW_AT_comp_dir,
                        AttributeValue::String(dir_bytes.to_vec()),
                    );
                    root.set(constants::DW_AT_stmt_list, AttributeValue::LineProgramRef);

                    let child = unit.add(unit.root(), constants::DW_TAG_subprogram);
                    unit.get_mut(child).set(
                        constants::DW_AT_decl_file,
                        AttributeValue::FileIndex(Some(file1)),
                    );

                    let child = unit.add(unit.root(), constants::DW_TAG_subprogram);
                    unit.get_mut(child).set(
                        constants::DW_AT_call_file,
                        AttributeValue::FileIndex(Some(file2)),
                    );

                    let mut dwarf = Dwarf::new();
                    dwarf.units.add(unit);

                    let mut sections = Sections::new(EndianVec::new(LittleEndian));
                    dwarf.write(&mut sections).unwrap();

                    let read_dwarf = sections.read(LittleEndian);
                    let mut read_units = read_dwarf.units();
                    let read_unit = read_units.next().unwrap().unwrap();
                    let read_unit = read_dwarf.unit(read_unit).unwrap();
                    let read_unit = read_unit.unit_ref(&read_dwarf);
                    let read_line_program = read_unit.line_program.as_ref().unwrap().header();
                    let mut read_entries = read_unit.entries();
                    let (_, _root) = read_entries.next_dfs().unwrap().unwrap();

                    let mut get_path = |name| {
                        let (_, entry) = read_entries.next_dfs().unwrap().unwrap();
                        let read_attr = entry.attr(name).unwrap().unwrap();
                        let read::AttributeValue::FileIndex(read_file_index) = read_attr.value()
                        else {
                            panic!("unexpected {:?}", read_attr);
                        };
                        let read_file = read_line_program.file(read_file_index).unwrap();
                        let read_path = read_unit
                            .attr_string(read_file.path_name())
                            .unwrap()
                            .slice();
                        (read_file_index, read_path)
                    };

                    let (read_index, read_path) = get_path(constants::DW_AT_decl_file);
                    assert_eq!(read_index, if version >= 5 { 0 } else { 1 });
                    assert_eq!(read_path, file_bytes1);

                    let (read_index, read_path) = get_path(constants::DW_AT_call_file);
                    assert_eq!(read_index, if version >= 5 { 1 } else { 2 });
                    assert_eq!(read_path, file_bytes2);

                    let convert_dwarf =
                        Dwarf::from(&read_dwarf, &|address| Some(Address::Constant(address)))
                            .unwrap();
                    let convert_unit = convert_dwarf.units.get(convert_dwarf.units.id(0));
                    let convert_root = convert_unit.get(convert_unit.root());
                    let mut convert_entries = convert_root.children();

                    let mut get_convert_path = |name| {
                        let convert_entry = convert_unit.get(*convert_entries.next().unwrap());
                        let convert_attr = convert_entry.get(name).unwrap();
                        let AttributeValue::FileIndex(Some(convert_file_index)) = convert_attr
                        else {
                            panic!("unexpected {:?}", convert_attr);
                        };
                        convert_unit.line_program.get_file(*convert_file_index).0
                    };

                    let convert_path = get_convert_path(constants::DW_AT_decl_file);
                    assert_eq!(convert_path, &file_string1);

                    let convert_path = get_convert_path(constants::DW_AT_call_file);
                    assert_eq!(convert_path, &file_string2);
                }
            }
        }
    }

    #[test]
    fn test_line_program_used() {
        for used in [false, true] {
            let encoding = Encoding {
                format: Format::Dwarf32,
                version: 5,
                address_size: 8,
            };

            let line_program = LineProgram::new(
                encoding,
                LineEncoding::default(),
                LineString::String(b"comp_dir".to_vec()),
                None,
                LineString::String(b"comp_name".to_vec()),
                None,
            );

            let mut unit = Unit::new(encoding, line_program);
            let file_id = if used { Some(FileId::new(0)) } else { None };
            let root = unit.root();
            unit.get_mut(root).set(
                constants::DW_AT_decl_file,
                AttributeValue::FileIndex(file_id),
            );

            let mut dwarf = Dwarf::new();
            dwarf.units.add(unit);

            let mut sections = Sections::new(EndianVec::new(LittleEndian));
            dwarf.write(&mut sections).unwrap();
            assert_eq!(!used, sections.debug_line.slice().is_empty());
        }
    }

    #[test]
    fn test_delete_child() {
        fn set_name(unit: &mut Unit, id: UnitEntryId, name: &str) {
            let entry = unit.get_mut(id);
            entry.set(constants::DW_AT_name, AttributeValue::String(name.into()));
        }
        fn check_name<R: read::Reader>(
            entry: &read::DebuggingInformationEntry<'_, '_, R>,
            unit: read::UnitRef<'_, R>,
            name: &str,
        ) {
            let name_attr = entry.attr(constants::DW_AT_name).unwrap().unwrap();
            let entry_name = unit.attr_string(name_attr.value()).unwrap();
            let entry_name_str = entry_name.to_string().unwrap();
            assert_eq!(entry_name_str, name);
        }
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 4,
            address_size: 8,
        };
        let mut dwarf = DwarfUnit::new(encoding);
        let root = dwarf.unit.root();

        // Add and delete entries in the root unit
        let child1 = dwarf.unit.add(root, constants::DW_TAG_subprogram);
        set_name(&mut dwarf.unit, child1, "child1");
        let grandchild1 = dwarf.unit.add(child1, constants::DW_TAG_variable);
        set_name(&mut dwarf.unit, grandchild1, "grandchild1");
        let child2 = dwarf.unit.add(root, constants::DW_TAG_subprogram);
        set_name(&mut dwarf.unit, child2, "child2");
        // This deletes both `child1` and its child `grandchild1`
        dwarf.unit.get_mut(root).delete_child(child1);
        let child3 = dwarf.unit.add(root, constants::DW_TAG_subprogram);
        set_name(&mut dwarf.unit, child3, "child3");
        let child4 = dwarf.unit.add(root, constants::DW_TAG_subprogram);
        set_name(&mut dwarf.unit, child4, "child4");
        let grandchild4 = dwarf.unit.add(child4, constants::DW_TAG_variable);
        set_name(&mut dwarf.unit, grandchild4, "grandchild4");
        dwarf.unit.get_mut(child4).delete_child(grandchild4);

        let mut sections = Sections::new(EndianVec::new(LittleEndian));

        // Write DWARF data which should only include `child2`, `child3` and `child4`
        dwarf.write(&mut sections).unwrap();

        let read_dwarf = sections.read(LittleEndian);
        let read_unit = read_dwarf.units().next().unwrap().unwrap();
        let read_unit = read_dwarf.unit(read_unit).unwrap();
        let read_unit = read_unit.unit_ref(&read_dwarf);
        let mut entries = read_unit.entries();
        // root
        entries.next_dfs().unwrap().unwrap();
        // child2
        let (_, read_child2) = entries.next_dfs().unwrap().unwrap();
        check_name(read_child2, read_unit, "child2");
        // child3
        let (_, read_child3) = entries.next_dfs().unwrap().unwrap();
        check_name(read_child3, read_unit, "child3");
        // child4
        let (_, read_child4) = entries.next_dfs().unwrap().unwrap();
        check_name(read_child4, read_unit, "child4");
        // There should be no more entries
        assert!(entries.next_dfs().unwrap().is_none());
    }

    #[test]
    fn test_missing_unit_ref() {
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 5,
            address_size: 8,
        };

        let mut dwarf = Dwarf::new();
        let unit_id = dwarf.units.add(Unit::new(encoding, LineProgram::none()));
        let unit = dwarf.units.get_mut(unit_id);

        // Create the entry to be referenced.
        let entry_id = unit.add(unit.root(), constants::DW_TAG_const_type);
        // And delete it so that it is not available when writing.
        unit.get_mut(unit.root()).delete_child(entry_id);

        // Create a reference to the deleted entry.
        let subprogram_id = unit.add(unit.root(), constants::DW_TAG_subprogram);
        unit.get_mut(subprogram_id)
            .set(constants::DW_AT_type, AttributeValue::UnitRef(entry_id));

        // Writing the DWARF should fail.
        let mut sections = Sections::new(EndianVec::new(LittleEndian));
        assert_eq!(dwarf.write(&mut sections), Err(Error::InvalidReference));
    }

    #[test]
    fn test_missing_debuginfo_ref() {
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 5,
            address_size: 8,
        };

        let mut dwarf = Dwarf::new();
        let unit_id = dwarf.units.add(Unit::new(encoding, LineProgram::none()));
        let unit = dwarf.units.get_mut(unit_id);

        // Create the entry to be referenced.
        let entry_id = unit.add(unit.root(), constants::DW_TAG_const_type);
        // And delete it so that it is not available when writing.
        unit.get_mut(unit.root()).delete_child(entry_id);

        // Create a reference to the deleted entry.
        let subprogram_id = unit.add(unit.root(), constants::DW_TAG_subprogram);
        unit.get_mut(subprogram_id).set(
            constants::DW_AT_type,
            AttributeValue::DebugInfoRef(Reference::Entry(unit_id, entry_id)),
        );

        // Writing the DWARF should fail.
        let mut sections = Sections::new(EndianVec::new(LittleEndian));
        assert_eq!(dwarf.write(&mut sections), Err(Error::InvalidReference));
    }
}
