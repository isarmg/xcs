#[cfg(test)]
mod tests {
    use crate::module::debug::units::DebuggingInformationCursor;

    use super::AddressSearchPreference;
    use gimli::*;
    use std::cell::RefCell;

    fn make_test_debug_line(
        debug_line: &mut write::DebugLine<write::EndianVec<LittleEndian>>,
    ) -> IncompleteLineProgram<EndianSlice<'_, LittleEndian>> {
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 4,
            address_size: 4,
        };
        let dir1 = &b"dir1"[..];
        let file1 = &b"file1"[..];
        let comp_file = write::LineString::String(file1.to_vec());

        let mut program = write::LineProgram::new(
            encoding,
            LineEncoding {
                minimum_instruction_length: 1,
                maximum_operations_per_instruction: 8,
                line_base: 0,
                line_range: 10,
                default_is_stmt: true,
            },
            write::LineString::String(dir1.to_vec()),
            None,
            comp_file.clone(),
            None,
        );

        {
            program.row().file = program.add_file(comp_file, program.default_directory(), None);
            program.begin_sequence(Some(write::Address::Constant(0x1000)));
            program.generate_row();
            let address_offset = program.row().address_offset + 1u64;
            program.end_sequence(address_offset);
        }

        program
            .write(
                debug_line,
                encoding,
                &write::DebugLineStrOffsets::none(),
                &write::DebugStrOffsets::none(),
            )
            .unwrap();

        let debug_line = read::DebugLine::new(debug_line.slice(), LittleEndian);
        debug_line
            .program(DebugLineOffset(0), 4, None, None)
            .unwrap()
    }

    #[test]
    fn convert_callback_invoke() {
        let called_address_to_be_converted: RefCell<Vec<(u64, AddressSearchPreference)>> =
            RefCell::new(Vec::new());
        {
            let mut debug_line = write::DebugLine::from(write::EndianVec::new(LittleEndian));
            let incomplete_debug_line = make_test_debug_line(&mut debug_line);

            let convert_address = |address, preference| -> Option<write::Address> {
                called_address_to_be_converted
                    .borrow_mut()
                    .push((address, preference));
                Some(write::Address::Constant(address + 0x10))
            };

            let empty_debug_str = Default::default();
            let empty_debug_line_str = Default::default();

            let mut line_strings = write::LineStringTable::default();
            let mut strings = write::StringTable::default();
            let mut convert_context = crate::module::debug::ConvertContext::new(
                &empty_debug_str,
                &empty_debug_line_str,
                &mut strings,
                &mut line_strings,
                &convert_address,
            );
            convert_context
                .convert_line_program(incomplete_debug_line)
                .unwrap();
        }

        {
            let called_address_to_be_converted = called_address_to_be_converted.borrow();
            assert_eq!(called_address_to_be_converted.len(), 3);
            assert_eq!(
                called_address_to_be_converted[0],
                (0x1000, AddressSearchPreference::ExclusiveFunctionEnd)
            ); // begin sequence
            assert_eq!(
                called_address_to_be_converted[1],
                (0x1000, AddressSearchPreference::InclusiveFunctionEnd)
            ); // first line row
            assert_eq!(
                called_address_to_be_converted[2],
                (0x1001, AddressSearchPreference::InclusiveFunctionEnd)
            ); // end sequence
        }
    }

    #[test]
    fn test_converted_address() {
        let mut converted_debug_line = write::DebugLine::from(write::EndianVec::new(LittleEndian));
        {
            let mut debug_line = write::DebugLine::from(write::EndianVec::new(LittleEndian));
            let incomplete_debug_line = make_test_debug_line(&mut debug_line);

            let convert_address = |address, _| -> Option<write::Address> {
                Some(write::Address::Constant(address + 0x10))
            };

            let empty_dwarf = Dwarf::default();
            let mut line_strings = write::LineStringTable::default();
            let mut strings = write::StringTable::default();

            let mut convert_context = crate::module::debug::ConvertContext::new(
                &empty_dwarf.debug_str,
                &empty_dwarf.debug_line_str,
                &mut strings,
                &mut line_strings,
                &convert_address,
            );
            let converted_program = convert_context
                .convert_line_program(incomplete_debug_line)
                .unwrap();

            converted_program
                .write(
                    &mut converted_debug_line,
                    converted_program.encoding(),
                    &write::DebugLineStrOffsets::none(),
                    &write::DebugStrOffsets::none(),
                )
                .unwrap();
        }

        {
            let read_debug_line = read::DebugLine::new(converted_debug_line.slice(), LittleEndian);
            let read_program = read_debug_line
                .program(DebugLineOffset(0), 4, None, None)
                .unwrap();

            let mut rows = read_program.rows();
            let row = rows.next_row().unwrap().unwrap().1;
            assert_eq!(row.address(), 0x1010);
        }
    }

    #[test]
    fn test_converted_none_address() {
        let mut converted_debug_line = write::DebugLine::from(write::EndianVec::new(LittleEndian));
        {
            let mut debug_line = write::DebugLine::from(write::EndianVec::new(LittleEndian));
            let incomplete_debug_line = make_test_debug_line(&mut debug_line);

            let convert_address = |_, _| -> Option<write::Address> { None };

            let empty_dwarf = Dwarf::default();
            let mut line_strings = write::LineStringTable::default();
            let mut strings = write::StringTable::default();

            let mut convert_context = crate::module::debug::ConvertContext::new(
                &empty_dwarf.debug_str,
                &empty_dwarf.debug_line_str,
                &mut strings,
                &mut line_strings,
                &convert_address,
            );
            let converted_program = convert_context
                .convert_line_program(incomplete_debug_line)
                .unwrap();

            converted_program
                .write(
                    &mut converted_debug_line,
                    converted_program.encoding(),
                    &write::DebugLineStrOffsets::none(),
                    &write::DebugStrOffsets::none(),
                )
                .unwrap();
        }

        {
            let read_debug_line = read::DebugLine::new(converted_debug_line.slice(), LittleEndian);
            let read_program = read_debug_line
                .program(DebugLineOffset(0), 4, None, None)
                .unwrap();

            let mut rows = read_program.rows();
            let row = rows.next_row().unwrap();
            assert!(row.is_none());
        }
    }

    fn make_test_debug_info() -> write::Sections<write::EndianVec<LittleEndian>> {
        let mut unit_table = write::UnitTable::default();
        {
            let mut unit1 = write::Unit::new(
                Encoding {
                    version: 4,
                    address_size: 4,
                    format: Format::Dwarf32,
                },
                write::LineProgram::none(),
            );

            let root_id = unit1.root();
            let child1_id = unit1.add(root_id, constants::DW_TAG_subprogram);
            unit1.get_mut(child1_id).set(
                constants::DW_AT_low_pc,
                write::AttributeValue::Address(write::Address::Constant(0x1000)),
            );
            unit1.get_mut(child1_id).set(
                constants::DW_AT_high_pc,
                write::AttributeValue::Data4(0x100),
            );

            unit_table.add(unit1);
        }

        let mut sections = write::Sections::new(write::EndianVec::new(LittleEndian));
        unit_table
            .write(
                &mut sections,
                &write::DebugLineStrOffsets::none(),
                &write::DebugStrOffsets::none(),
            )
            .unwrap();
        sections
    }

    #[test]
    fn test_convert_high_pc() {
        let sections = make_test_debug_info();
        let read_dwarf = Dwarf {
            debug_info: read::DebugInfo::new(sections.debug_info.slice(), LittleEndian),
            debug_abbrev: read::DebugAbbrev::new(sections.debug_abbrev.slice(), LittleEndian),
            ..Default::default()
        };

        let read_first_unit_header = read_dwarf.units().next().unwrap().unwrap();
        let read_first_unit = read_dwarf.unit(read_first_unit_header).unwrap();
        let mut read_first_unit_entries = read_first_unit.entries();

        let convert_address = |address, _| -> Option<write::Address> {
            if address < 0x1050 {
                Some(write::Address::Constant(address + 0x10))
            } else {
                Some(write::Address::Constant(address - 0x10))
            }
        };

        let mut strings = write::StringTable::default();
        let mut line_strings = write::LineStringTable::default();

        let convert_context = crate::module::debug::ConvertContext::new(
            &read_dwarf.debug_str,
            &read_dwarf.debug_line_str,
            &mut strings,
            &mut line_strings,
            &convert_address,
        );

        let mut converted_dwarf = write::Dwarf::from(&read_dwarf, &|address| {
            convert_address(address, AddressSearchPreference::InclusiveFunctionEnd)
        })
        .unwrap();

        let id = converted_dwarf.units.id(0);

        {
            let unit = converted_dwarf.units.get_mut(id);
            let mut write_unit_first_entries = DebuggingInformationCursor::new(unit);

            convert_context
                .convert_high_pc(&mut read_first_unit_entries, &mut write_unit_first_entries);
        }

        {
            let unit = converted_dwarf.units.get_mut(id);
            let mut write_unit_first_entries = DebuggingInformationCursor::new(unit);

            let _root = write_unit_first_entries.next_dfs();
            let subprogram_entry = write_unit_first_entries.next_dfs().unwrap();

            assert_eq!(
                *subprogram_entry.get(DW_AT_low_pc).unwrap(),
                write::AttributeValue::Address(write::Address::Constant(0x1010))
            );
            assert_eq!(
                *subprogram_entry.get(DW_AT_high_pc).unwrap(),
                write::AttributeValue::Data4(0xE0)
            );
        }

        // The emitted form must stay fixed-width, as in the input.
        let mut sections = write::Sections::new(write::EndianVec::new(LittleEndian));
        converted_dwarf.write(&mut sections).unwrap();
        let read_dwarf = Dwarf {
            debug_info: read::DebugInfo::new(sections.debug_info.slice(), LittleEndian),
            debug_abbrev: read::DebugAbbrev::new(sections.debug_abbrev.slice(), LittleEndian),
            ..Default::default()
        };
        let header = read_dwarf.units().next().unwrap().unwrap();
        let unit = read_dwarf.unit(header).unwrap();
        let mut entries = unit.entries();
        entries.next_dfs().unwrap();
        let (_, subprogram) = entries.next_dfs().unwrap().unwrap();
        let high_pc = subprogram.attr(DW_AT_high_pc).unwrap().unwrap();
        assert_eq!(high_pc.raw_value(), AttributeValue::Data4(0xE0));
    }
}
