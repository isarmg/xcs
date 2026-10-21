#[cfg(test)]
#[cfg(feature = "read")]
mod tests {
    use super::*;
    use crate::read;
    use crate::write::{AttributeValue, Dwarf, EndianVec, Sections, Unit};
    use crate::LittleEndian;

    #[test]
    fn test_line_program() {
        let dir1 = LineString::String(b"dir1".to_vec());
        let file1 = LineString::String(b"file1".to_vec());
        let dir2 = LineString::String(b"dir2".to_vec());
        let file2 = LineString::String(b"file2".to_vec());

        let mut dwarf = Dwarf::new();

        for &version in &[2, 3, 4, 5] {
            for &address_size in &[4, 8] {
                for &format in &[Format::Dwarf32, Format::Dwarf64] {
                    let encoding = Encoding {
                        format,
                        version,
                        address_size,
                    };
                    let mut program = LineProgram::new(
                        encoding,
                        LineEncoding::default(),
                        dir1.clone(),
                        None,
                        file1.clone(),
                        None,
                    );

                    assert_eq!(&dir1, program.get_directory(program.default_directory()));
                    program.file_has_timestamp = true;
                    program.file_has_size = true;
                    if encoding.version >= 5 {
                        program.file_has_md5 = true;
                    }

                    // Note: Embedded source code is an accepted extension
                    // that will become part of DWARF v6. We're using the LLVM extension
                    // here for v5.
                    if encoding.version >= 5 {
                        program.file_has_source = true;
                    }

                    let dir_id = program.add_directory(dir2.clone());
                    assert_eq!(&dir2, program.get_directory(dir_id));
                    assert_eq!(dir_id, program.add_directory(dir2.clone()));

                    let file_info = FileInfo {
                        timestamp: 1,
                        size: 2,
                        md5: if encoding.version >= 5 {
                            [3; 16]
                        } else {
                            [0; 16]
                        },
                        source: (encoding.version >= 5)
                            .then(|| LineString::String(b"the source code\n".to_vec())),
                    };
                    let file_id = program.add_file(file2.clone(), dir_id, Some(file_info.clone()));
                    assert_eq!((&file2, dir_id), program.get_file(file_id));
                    assert_eq!(file_info, *program.get_file_info(file_id));

                    program.get_file_info_mut(file_id).size = 3;
                    assert_ne!(file_info, *program.get_file_info(file_id));
                    assert_eq!(file_id, program.add_file(file2.clone(), dir_id, None));
                    assert_ne!(file_info, *program.get_file_info(file_id));
                    assert_eq!(
                        file_id,
                        program.add_file(file2.clone(), dir_id, Some(file_info.clone()))
                    );
                    assert_eq!(file_info, *program.get_file_info(file_id));

                    let mut unit = Unit::new(encoding, program);
                    let root = unit.get_mut(unit.root());
                    root.set(
                        constants::DW_AT_comp_dir,
                        AttributeValue::String(b"dir1".to_vec()),
                    );
                    root.set(
                        constants::DW_AT_name,
                        AttributeValue::String(b"file1".to_vec()),
                    );
                    root.set(constants::DW_AT_stmt_list, AttributeValue::LineProgramRef);
                    root.set(
                        constants::DW_AT_decl_file,
                        AttributeValue::FileIndex(Some(file_id)),
                    );

                    dwarf.units.add(unit);
                }
            }
        }

        let mut sections = Sections::new(EndianVec::new(LittleEndian));
        dwarf.write(&mut sections).unwrap();
        let read_dwarf = sections.read(LittleEndian);
        let convert_dwarf =
            Dwarf::from(&read_dwarf, &|address| Some(Address::Constant(address))).unwrap();

        let mut convert_units = convert_dwarf.units.iter();
        for (_, unit) in dwarf.units.iter() {
            let program = &unit.line_program;
            let root = unit.get(unit.root());
            let Some(AttributeValue::FileIndex(Some(file_id))) =
                root.get(constants::DW_AT_decl_file)
            else {
                panic!("missing DW_AT_decl_file");
            };

            let (_, convert_unit) = convert_units.next().unwrap();
            let convert_program = &convert_unit.line_program;
            let convert_root = convert_unit.get(convert_unit.root());
            let Some(AttributeValue::FileIndex(Some(convert_file_id))) =
                convert_root.get(constants::DW_AT_decl_file)
            else {
                panic!("missing DW_AT_decl_file");
            };

            assert_eq!(convert_program.version(), program.version());
            assert_eq!(convert_program.address_size(), program.address_size());
            assert_eq!(convert_program.format(), program.format());

            let (file, dir) = program.get_file(*file_id);
            let (convert_file, convert_dir) = convert_program.get_file(*convert_file_id);
            assert_eq!(file, convert_file);
            assert_eq!(
                program.get_directory(dir),
                convert_program.get_directory(convert_dir)
            );
            assert_eq!(
                program.get_file_info(*file_id),
                convert_program.get_file_info(*convert_file_id)
            );
        }
    }

    #[test]
    fn test_line_row() {
        let dir1 = &b"dir1"[..];
        let file1 = &b"file1"[..];
        let file2 = &b"file2"[..];

        for &version in &[2, 3, 4, 5] {
            for &address_size in &[4, 8] {
                for &format in &[Format::Dwarf32, Format::Dwarf64] {
                    let encoding = Encoding {
                        format,
                        version,
                        address_size,
                    };
                    let line_base = -5;
                    let line_range = 14;
                    let neg_line_base = (-line_base) as u8;
                    let mut program = LineProgram::new(
                        encoding,
                        LineEncoding {
                            line_base,
                            line_range,
                            ..Default::default()
                        },
                        LineString::String(dir1.to_vec()),
                        None,
                        LineString::String(file1.to_vec()),
                        None,
                    );
                    let dir_id = program.default_directory();
                    let file1_id =
                        program.add_file(LineString::String(file1.to_vec()), dir_id, None);
                    let file2_id =
                        program.add_file(LineString::String(file2.to_vec()), dir_id, None);

                    // Test sequences.
                    {
                        let mut program = program.clone();
                        let address = Address::Constant(0x12);
                        program.begin_sequence(Some(address));
                        assert_eq!(
                            program.instructions,
                            vec![LineInstruction::SetAddress(address)]
                        );
                    }

                    {
                        let mut program = program.clone();
                        program.begin_sequence(None);
                        assert_eq!(program.instructions, Vec::new());
                    }

                    {
                        let mut program = program.clone();
                        program.begin_sequence(None);
                        program.end_sequence(0x1234);
                        assert_eq!(
                            program.instructions,
                            vec![
                                LineInstruction::AdvancePc(0x1234),
                                LineInstruction::EndSequence
                            ]
                        );
                    }

                    // Create a base program.
                    program.begin_sequence(None);
                    program.row.line = 0x1000;
                    program.generate_row();
                    let base_row = program.row;
                    let base_instructions = program.instructions.clone();

                    // Create test cases.
                    let mut tests = Vec::new();

                    let row = base_row;
                    tests.push((row, vec![LineInstruction::Copy]));

                    let mut row = base_row;
                    row.line -= u64::from(neg_line_base);
                    tests.push((row, vec![LineInstruction::Special(OPCODE_BASE)]));

                    let mut row = base_row;
                    row.line += u64::from(line_range) - 1;
                    row.line -= u64::from(neg_line_base);
                    tests.push((
                        row,
                        vec![LineInstruction::Special(OPCODE_BASE + line_range - 1)],
                    ));

                    let mut row = base_row;
                    row.line += u64::from(line_range);
                    row.line -= u64::from(neg_line_base);
                    tests.push((
                        row,
                        vec![
                            LineInstruction::AdvanceLine(i64::from(line_range - neg_line_base)),
                            LineInstruction::Copy,
                        ],
                    ));

                    let mut row = base_row;
                    row.address_offset = 1;
                    row.line -= u64::from(neg_line_base);
                    tests.push((
                        row,
                        vec![LineInstruction::Special(OPCODE_BASE + line_range)],
                    ));

                    let op_range = (255 - OPCODE_BASE) / line_range;
                    let mut row = base_row;
                    row.address_offset = u64::from(op_range);
                    row.line -= u64::from(neg_line_base);
                    tests.push((
                        row,
                        vec![LineInstruction::Special(
                            OPCODE_BASE + op_range * line_range,
                        )],
                    ));

                    let mut row = base_row;
                    row.address_offset = u64::from(op_range);
                    row.line += u64::from(255 - OPCODE_BASE - op_range * line_range);
                    row.line -= u64::from(neg_line_base);
                    tests.push((row, vec![LineInstruction::Special(255)]));

                    let mut row = base_row;
                    row.address_offset = u64::from(op_range);
                    row.line += u64::from(255 - OPCODE_BASE - op_range * line_range) + 1;
                    row.line -= u64::from(neg_line_base);
                    tests.push((
                        row,
                        vec![LineInstruction::ConstAddPc, LineInstruction::Copy],
                    ));

                    let mut row = base_row;
                    row.address_offset = u64::from(op_range);
                    row.line += u64::from(255 - OPCODE_BASE - op_range * line_range) + 2;
                    row.line -= u64::from(neg_line_base);
                    tests.push((
                        row,
                        vec![
                            LineInstruction::ConstAddPc,
                            LineInstruction::Special(OPCODE_BASE + 6),
                        ],
                    ));

                    let mut row = base_row;
                    row.address_offset = u64::from(op_range) * 2;
                    row.line += u64::from(255 - OPCODE_BASE - op_range * line_range);
                    row.line -= u64::from(neg_line_base);
                    tests.push((
                        row,
                        vec![LineInstruction::ConstAddPc, LineInstruction::Special(255)],
                    ));

                    let mut row = base_row;
                    row.address_offset = u64::from(op_range) * 2;
                    row.line += u64::from(255 - OPCODE_BASE - op_range * line_range) + 1;
                    row.line -= u64::from(neg_line_base);
                    tests.push((
                        row,
                        vec![
                            LineInstruction::AdvancePc(row.address_offset),
                            LineInstruction::Copy,
                        ],
                    ));

                    let mut row = base_row;
                    row.address_offset = u64::from(op_range) * 2;
                    row.line += u64::from(255 - OPCODE_BASE - op_range * line_range) + 2;
                    row.line -= u64::from(neg_line_base);
                    tests.push((
                        row,
                        vec![
                            LineInstruction::AdvancePc(row.address_offset),
                            LineInstruction::Special(OPCODE_BASE + 6),
                        ],
                    ));

                    let mut row = base_row;
                    row.address_offset = 0x1234;
                    tests.push((
                        row,
                        vec![LineInstruction::AdvancePc(0x1234), LineInstruction::Copy],
                    ));

                    let mut row = base_row;
                    row.line += 0x1234;
                    tests.push((
                        row,
                        vec![LineInstruction::AdvanceLine(0x1234), LineInstruction::Copy],
                    ));

                    let mut row = base_row;
                    row.file = file1_id;
                    if version == 5 {
                        // Version 5 is 0-based, but the default file is 1, so this row
                        // will need to set the file.
                        tests.push((
                            row,
                            vec![LineInstruction::SetFile(file1_id), LineInstruction::Copy],
                        ));
                    } else {
                        // This is the first file, so normally this is already the default.
                        tests.push((row, vec![LineInstruction::Copy]));
                    }

                    let mut row = base_row;
                    row.file = file2_id;
                    if version == 5 {
                        tests.push((row, vec![LineInstruction::Copy]));
                    } else {
                        tests.push((
                            row,
                            vec![LineInstruction::SetFile(file2_id), LineInstruction::Copy],
                        ));
                    }

                    let mut row = base_row;
                    row.column = 0x1234;
                    tests.push((
                        row,
                        vec![LineInstruction::SetColumn(0x1234), LineInstruction::Copy],
                    ));

                    let mut row = base_row;
                    row.discriminator = 0x1234;
                    tests.push((
                        row,
                        vec![
                            LineInstruction::SetDiscriminator(0x1234),
                            LineInstruction::Copy,
                        ],
                    ));

                    let mut row = base_row;
                    row.is_statement = !row.is_statement;
                    tests.push((
                        row,
                        vec![LineInstruction::NegateStatement, LineInstruction::Copy],
                    ));

                    let mut row = base_row;
                    row.basic_block = true;
                    tests.push((
                        row,
                        vec![LineInstruction::SetBasicBlock, LineInstruction::Copy],
                    ));

                    let mut row = base_row;
                    row.prologue_end = true;
                    tests.push((
                        row,
                        vec![LineInstruction::SetPrologueEnd, LineInstruction::Copy],
                    ));

                    let mut row = base_row;
                    row.epilogue_begin = true;
                    tests.push((
                        row,
                        vec![LineInstruction::SetEpilogueBegin, LineInstruction::Copy],
                    ));

                    let mut row = base_row;
                    row.isa = 0x1234;
                    tests.push((
                        row,
                        vec![LineInstruction::SetIsa(0x1234), LineInstruction::Copy],
                    ));

                    for test in tests {
                        // Test generate_row().
                        let mut program = program.clone();
                        program.row = test.0;
                        program.generate_row();
                        assert_eq!(
                            &program.instructions[base_instructions.len()..],
                            &test.1[..]
                        );

                        // Test LineProgram::from().
                        let mut unit = Unit::new(encoding, program);
                        let root = unit.get_mut(unit.root());
                        root.set(constants::DW_AT_stmt_list, AttributeValue::LineProgramRef);

                        let mut dwarf = Dwarf::new();
                        dwarf.units.add(unit);

                        let mut sections = Sections::new(EndianVec::new(LittleEndian));
                        dwarf.write(&mut sections).unwrap();
                        let read_dwarf = sections.read(LittleEndian);

                        let convert_dwarf =
                            Dwarf::from(&read_dwarf, &|address| Some(Address::Constant(address)))
                                .unwrap();
                        let convert_unit = convert_dwarf.units.iter().next().unwrap().1;
                        let convert_program = &convert_unit.line_program;

                        assert_eq!(
                            &convert_program.instructions[base_instructions.len()..],
                            &test.1[..]
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_line_instruction() {
        let dir1 = &b"dir1"[..];
        let file1 = &b"file1"[..];

        for &version in &[2, 3, 4, 5] {
            for &address_size in &[4, 8] {
                for &format in &[Format::Dwarf32, Format::Dwarf64] {
                    let encoding = Encoding {
                        format,
                        version,
                        address_size,
                    };
                    let mut program = LineProgram::new(
                        encoding,
                        LineEncoding::default(),
                        LineString::String(dir1.to_vec()),
                        None,
                        LineString::String(file1.to_vec()),
                        None,
                    );
                    let dir_id = program.default_directory();
                    let file_id =
                        program.add_file(LineString::String(file1.to_vec()), dir_id, None);

                    for (inst, expect_inst) in &[
                        (
                            LineInstruction::Special(OPCODE_BASE),
                            read::LineInstruction::Special(OPCODE_BASE),
                        ),
                        (
                            LineInstruction::Special(255),
                            read::LineInstruction::Special(255),
                        ),
                        (LineInstruction::Copy, read::LineInstruction::Copy),
                        (
                            LineInstruction::AdvancePc(0x12),
                            read::LineInstruction::AdvancePc(0x12),
                        ),
                        (
                            LineInstruction::AdvanceLine(0x12),
                            read::LineInstruction::AdvanceLine(0x12),
                        ),
                        (
                            LineInstruction::SetFile(file_id),
                            read::LineInstruction::SetFile(file_id.raw(encoding.version)),
                        ),
                        (
                            LineInstruction::SetColumn(0x12),
                            read::LineInstruction::SetColumn(0x12),
                        ),
                        (
                            LineInstruction::NegateStatement,
                            read::LineInstruction::NegateStatement,
                        ),
                        (
                            LineInstruction::SetBasicBlock,
                            read::LineInstruction::SetBasicBlock,
                        ),
                        (
                            LineInstruction::ConstAddPc,
                            read::LineInstruction::ConstAddPc,
                        ),
                        (
                            LineInstruction::SetPrologueEnd,
                            read::LineInstruction::SetPrologueEnd,
                        ),
                        (
                            LineInstruction::SetEpilogueBegin,
                            read::LineInstruction::SetEpilogueBegin,
                        ),
                        (
                            LineInstruction::SetIsa(0x12),
                            read::LineInstruction::SetIsa(0x12),
                        ),
                        (
                            LineInstruction::EndSequence,
                            read::LineInstruction::EndSequence,
                        ),
                        (
                            LineInstruction::SetAddress(Address::Constant(0x12)),
                            read::LineInstruction::SetAddress(0x12),
                        ),
                        (
                            LineInstruction::SetDiscriminator(0x12),
                            read::LineInstruction::SetDiscriminator(0x12),
                        ),
                    ][..]
                    {
                        let mut program = program.clone();
                        program.instructions.push(*inst);

                        let mut unit = Unit::new(encoding, program);
                        let root = unit.get_mut(unit.root());
                        root.set(constants::DW_AT_stmt_list, AttributeValue::LineProgramRef);

                        let mut dwarf = Dwarf::new();
                        dwarf.units.add(unit);
                        let mut sections = Sections::new(EndianVec::new(LittleEndian));
                        dwarf.write(&mut sections).unwrap();

                        let read_dwarf = sections.read(LittleEndian);
                        let read_unit_header = read_dwarf.units().next().unwrap().unwrap();
                        let read_unit = read_dwarf.unit(read_unit_header).unwrap();
                        let read_unit = read_unit.unit_ref(&read_dwarf);
                        let read_header = read_unit.line_program.as_ref().unwrap().header();
                        let mut read_insts = read_header.instructions();
                        assert_eq!(
                            *expect_inst,
                            read_insts.next_instruction(read_header).unwrap().unwrap()
                        );
                        assert_eq!(None, read_insts.next_instruction(read_header).unwrap());
                    }
                }
            }
        }
    }

    // Test that the address/line advance is correct. We don't test for optimality.
    #[test]
    fn test_advance() {
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 4,
            address_size: 8,
        };

        let dir1 = &b"dir1"[..];
        let file1 = &b"file1"[..];

        let addresses = 0..50;
        let lines = -10..25i64;

        for minimum_instruction_length in [1, 4] {
            for maximum_operations_per_instruction in [1, 3] {
                for line_base in [-5, 0] {
                    for line_range in [10, 20] {
                        let line_encoding = LineEncoding {
                            minimum_instruction_length,
                            maximum_operations_per_instruction,
                            line_base,
                            line_range,
                            default_is_stmt: true,
                        };
                        let mut program = LineProgram::new(
                            encoding,
                            line_encoding,
                            LineString::String(dir1.to_vec()),
                            None,
                            LineString::String(file1.to_vec()),
                            None,
                        );
                        for address_advance in addresses.clone() {
                            program.begin_sequence(Some(Address::Constant(0x1000)));
                            program.row().line = 0x10000;
                            program.generate_row();
                            for line_advance in lines.clone() {
                                {
                                    let row = program.row();
                                    row.address_offset +=
                                        address_advance * u64::from(minimum_instruction_length);
                                    row.line = row.line.wrapping_add(line_advance as u64);
                                }
                                program.generate_row();
                            }
                            let address_offset = program.row().address_offset
                                + u64::from(minimum_instruction_length);
                            program.end_sequence(address_offset);
                        }

                        let mut unit = Unit::new(encoding, program);
                        let root = unit.get_mut(unit.root());
                        root.set(constants::DW_AT_stmt_list, AttributeValue::LineProgramRef);

                        let mut dwarf = Dwarf::new();
                        dwarf.units.add(unit);
                        let mut sections = Sections::new(EndianVec::new(LittleEndian));
                        dwarf.write(&mut sections).unwrap();

                        let read_dwarf = sections.read(LittleEndian);
                        let read_unit_header = read_dwarf.units().next().unwrap().unwrap();
                        let read_unit = read_dwarf.unit(read_unit_header).unwrap();
                        let read_unit = read_unit.unit_ref(&read_dwarf);
                        let read_program = read_unit.line_program.clone().unwrap();

                        let mut rows = read_program.rows();
                        for address_advance in addresses.clone() {
                            let mut address;
                            let mut line;
                            {
                                let row = rows.next_row().unwrap().unwrap().1;
                                address = row.address();
                                line = row.line().unwrap().get();
                            }
                            assert_eq!(address, 0x1000);
                            assert_eq!(line, 0x10000);
                            for line_advance in lines.clone() {
                                let row = rows.next_row().unwrap().unwrap().1;
                                assert_eq!(
                                    row.address() - address,
                                    address_advance * u64::from(minimum_instruction_length)
                                );
                                assert_eq!(
                                    (row.line().unwrap().get() as i64) - (line as i64),
                                    line_advance
                                );
                                address = row.address();
                                line = row.line().unwrap().get();
                            }
                            let row = rows.next_row().unwrap().unwrap().1;
                            assert!(row.end_sequence());
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_line_string() {
        let version = 5;

        let file1 = "file1";

        for &address_size in &[4, 8] {
            for &format in &[Format::Dwarf32, Format::Dwarf64] {
                let encoding = Encoding {
                    format,
                    version,
                    address_size,
                };

                let files: &mut [&mut dyn Fn(&mut Dwarf) -> LineString] = &mut [
                    &mut |_dwarf| LineString::String(file1.as_bytes().to_vec()),
                    &mut |dwarf| LineString::StringRef(dwarf.strings.add(file1)),
                    &mut |dwarf| LineString::LineStringRef(dwarf.line_strings.add(file1)),
                ];

                for file in files {
                    let mut dwarf = Dwarf::new();
                    let file = file(&mut dwarf);

                    let mut program = LineProgram::new(
                        encoding,
                        LineEncoding::default(),
                        LineString::String(b"dir".to_vec()),
                        None,
                        file.clone(),
                        None,
                    );
                    program.begin_sequence(Some(Address::Constant(0x1000)));
                    program.row().line = 0x10000;
                    program.generate_row();

                    let mut unit = Unit::new(encoding, program);
                    let root = unit.get_mut(unit.root());
                    root.set(constants::DW_AT_stmt_list, AttributeValue::LineProgramRef);
                    dwarf.units.add(unit);

                    let mut sections = Sections::new(EndianVec::new(LittleEndian));
                    dwarf.write(&mut sections).unwrap();

                    let read_dwarf = sections.read(LittleEndian);
                    let read_unit_header = read_dwarf.units().next().unwrap().unwrap();
                    let read_unit = read_dwarf.unit(read_unit_header).unwrap();
                    let read_unit = read_unit.unit_ref(&read_dwarf);
                    let read_program = read_unit.line_program.clone().unwrap();
                    let read_header = read_program.header();
                    let read_file = read_header.file(0).unwrap();
                    let read_path = read_unit.attr_string(read_file.path_name()).unwrap();
                    assert_eq!(read_path.slice(), file1.as_bytes());
                }
            }
        }
    }

    #[test]
    fn test_missing_comp_dir() {
        for &version in &[2, 3, 4, 5] {
            for &address_size in &[4, 8] {
                for &format in &[Format::Dwarf32, Format::Dwarf64] {
                    let encoding = Encoding {
                        format,
                        version,
                        address_size,
                    };
                    let mut program = LineProgram::new(
                        encoding,
                        LineEncoding::default(),
                        LineString::String(Vec::new()),
                        None,
                        LineString::String(Vec::new()),
                        None,
                    );
                    // Ensure the program is not empty.
                    let dir_id = program.default_directory();
                    let file_id =
                        program.add_file(LineString::String(b"file1".to_vec()), dir_id, None);
                    program.begin_sequence(Some(Address::Constant(0x1000)));
                    program.row().file = file_id;
                    program.row().line = 0x10000;
                    program.generate_row();

                    let mut unit = Unit::new(encoding, program);
                    let root = unit.get_mut(unit.root());
                    // Testing missing DW_AT_comp_dir/DW_AT_name.
                    root.set(constants::DW_AT_stmt_list, AttributeValue::LineProgramRef);

                    let mut dwarf = Dwarf::new();
                    dwarf.units.add(unit);
                    let mut sections = Sections::new(EndianVec::new(LittleEndian));
                    dwarf.write(&mut sections).unwrap();
                    let read_dwarf = sections.read(LittleEndian);
                    let _convert_dwarf =
                        Dwarf::from(&read_dwarf, &|address| Some(Address::Constant(address)))
                            .unwrap();
                }
            }
        }
    }

    #[test]
    fn test_separate_working_dir() {
        let working_dir = LineString::String(b"working".to_vec());
        let source_dir = LineString::String(b"source".to_vec());
        let source_file = LineString::String(b"file".to_vec());

        for &version in &[2, 3, 4, 5] {
            for &address_size in &[4, 8] {
                for &format in &[Format::Dwarf32, Format::Dwarf64] {
                    let encoding = Encoding {
                        format,
                        version,
                        address_size,
                    };
                    let mut program = LineProgram::new(
                        encoding,
                        LineEncoding::default(),
                        working_dir.clone(),
                        Some(source_dir.clone()),
                        source_file.clone(),
                        None,
                    );

                    assert_eq!(
                        &working_dir,
                        program.get_directory(program.default_directory())
                    );

                    // Ensure the program is not empty.
                    let dir_id = program.add_directory(source_dir.clone());
                    let file_id = program.add_file(source_file.clone(), dir_id, None);
                    program.begin_sequence(Some(Address::Constant(0x1000)));
                    program.row().file = file_id;
                    program.row().line = 0x10000;
                    program.generate_row();

                    // Test LineProgram::from().
                    let mut unit = Unit::new(encoding, program);
                    let root = unit.get_mut(unit.root());
                    root.set(
                        constants::DW_AT_comp_dir,
                        AttributeValue::String(b"working".to_vec()),
                    );
                    root.set(
                        constants::DW_AT_name,
                        AttributeValue::String(b"source/file".to_vec()),
                    );
                    root.set(constants::DW_AT_stmt_list, AttributeValue::LineProgramRef);

                    let mut dwarf = Dwarf::new();
                    dwarf.units.add(unit);

                    let mut sections = Sections::new(EndianVec::new(LittleEndian));
                    dwarf.write(&mut sections).unwrap();
                    let read_dwarf = sections.read(LittleEndian);

                    let convert_dwarf =
                        Dwarf::from(&read_dwarf, &|address| Some(Address::Constant(address)))
                            .unwrap();
                    let convert_unit = convert_dwarf.units.iter().next().unwrap().1;
                    let convert_program = &convert_unit.line_program;

                    assert_eq!(
                        &working_dir,
                        convert_program.get_directory(convert_program.default_directory())
                    );
                    let (_file_id, file, dir_id) = convert_program.files().next().unwrap();
                    assert_eq!(&source_file, file);
                    assert_eq!(&source_dir, convert_program.get_directory(dir_id));
                }
            }
        }
    }

    #[test]
    fn test_file_source() {
        let version = 5;

        let source1 = "source1";

        for &address_size in &[4, 8] {
            for &format in &[Format::Dwarf32, Format::Dwarf64] {
                let encoding = Encoding {
                    format,
                    version,
                    address_size,
                };

                let sources: &mut [&mut dyn Fn(&mut Dwarf) -> LineString] = &mut [
                    &mut |_dwarf| LineString::String(source1.as_bytes().to_vec()),
                    &mut |dwarf| LineString::StringRef(dwarf.strings.add(source1)),
                    &mut |dwarf| LineString::LineStringRef(dwarf.line_strings.add(source1)),
                ];

                for source in sources {
                    let mut dwarf = Dwarf::new();
                    let source = Some(source(&mut dwarf));

                    let mut program = LineProgram::new(
                        encoding,
                        LineEncoding::default(),
                        LineString::String(b"dir".to_vec()),
                        None,
                        LineString::String(b"file".to_vec()),
                        Some(FileInfo {
                            timestamp: 0,
                            size: 0,
                            md5: [0; 16],
                            source,
                        }),
                    );
                    program.file_has_source = true;

                    let file_id = program.files().next().unwrap().0;
                    program.begin_sequence(Some(Address::Constant(0x1000)));
                    program.row().file = file_id;
                    program.row().line = 0x10000;
                    program.generate_row();

                    let mut unit = Unit::new(encoding, program);
                    let root = unit.get_mut(unit.root());
                    root.set(constants::DW_AT_stmt_list, AttributeValue::LineProgramRef);
                    dwarf.units.add(unit);

                    let mut sections = Sections::new(EndianVec::new(LittleEndian));
                    dwarf.write(&mut sections).unwrap();

                    let read_dwarf = sections.read(LittleEndian);
                    let read_unit_header = read_dwarf.units().next().unwrap().unwrap();
                    let read_unit = read_dwarf.unit(read_unit_header).unwrap();
                    let read_unit = read_unit.unit_ref(&read_dwarf);
                    let read_program = read_unit.line_program.clone().unwrap();
                    let read_header = read_program.header();
                    let read_file = read_header.file(0).unwrap();
                    let read_source = read_unit.attr_string(read_file.source().unwrap()).unwrap();
                    assert_eq!(read_source.slice(), source1.as_bytes());

                    let convert_dwarf =
                        Dwarf::from(&read_dwarf, &|address| Some(Address::Constant(address)))
                            .unwrap();
                    let (_, convert_unit) = convert_dwarf.units.iter().next().unwrap();
                    let convert_program = &convert_unit.line_program;
                    let convert_file_id = convert_program.files().next().unwrap().0;
                    let convert_file_info = convert_program.get_file_info(convert_file_id);
                    assert_eq!(
                        convert_dwarf.get_line_string(convert_file_info.source.as_ref().unwrap()),
                        source1.as_bytes(),
                    );
                }
            }
        }
    }
}
