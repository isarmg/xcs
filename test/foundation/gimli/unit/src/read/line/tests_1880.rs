#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants;
    use crate::endianity::LittleEndian;
    use crate::read::{EndianSlice, Error};
    use crate::test_util::GimliSectionMethods;
    use test_assembler::{Endian, Label, LabelMaker, Section};

    #[test]
    fn test_parse_debug_line_32_ok() {
        #[rustfmt::skip]
        let buf = [
            // 32-bit length = 62.
            0x3e, 0x00, 0x00, 0x00,
            // Version.
            0x04, 0x00,
            // Header length = 40.
            0x28, 0x00, 0x00, 0x00,
            // Minimum instruction length.
            0x01,
            // Maximum operations per byte.
            0x01,
            // Default is_stmt.
            0x01,
            // Line base.
            0x00,
            // Line range.
            0x01,
            // Opcode base.
            0x03,
            // Standard opcode lengths for opcodes 1 .. opcode base - 1.
            0x01, 0x02,
            // Include directories = '/', 'i', 'n', 'c', '\0', '/', 'i', 'n', 'c', '2', '\0', '\0'
            0x2f, 0x69, 0x6e, 0x63, 0x00, 0x2f, 0x69, 0x6e, 0x63, 0x32, 0x00, 0x00,
            // File names
                // foo.rs
                0x66, 0x6f, 0x6f, 0x2e, 0x72, 0x73, 0x00,
                0x00,
                0x00,
                0x00,
                // bar.h
                0x62, 0x61, 0x72, 0x2e, 0x68, 0x00,
                0x01,
                0x00,
                0x00,
            // End file names.
            0x00,

            // Dummy line program data.
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,

            // Dummy next line program.
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];

        let rest = &mut EndianSlice::new(&buf, LittleEndian);
        let comp_dir = EndianSlice::new(b"/comp_dir", LittleEndian);
        let comp_name = EndianSlice::new(b"/comp_name", LittleEndian);

        let header =
            LineProgramHeader::parse(rest, DebugLineOffset(0), 4, Some(comp_dir), Some(comp_name))
                .expect("should parse header ok");

        assert_eq!(
            *rest,
            EndianSlice::new(&buf[buf.len() - 16..], LittleEndian)
        );

        assert_eq!(header.offset, DebugLineOffset(0));
        assert_eq!(header.version(), 4);
        assert_eq!(header.minimum_instruction_length(), 1);
        assert_eq!(header.maximum_operations_per_instruction(), 1);
        assert!(header.default_is_stmt());
        assert_eq!(header.line_base(), 0);
        assert_eq!(header.line_range(), 1);
        assert_eq!(header.opcode_base(), 3);
        assert_eq!(header.directory(0), Some(AttributeValue::String(comp_dir)));
        assert_eq!(
            header.file(0).unwrap().path_name,
            AttributeValue::String(comp_name)
        );

        let expected_lengths = [1, 2];
        assert_eq!(header.standard_opcode_lengths().slice(), &expected_lengths);

        let expected_include_directories = [
            AttributeValue::String(EndianSlice::new(b"/inc", LittleEndian)),
            AttributeValue::String(EndianSlice::new(b"/inc2", LittleEndian)),
        ];
        assert_eq!(header.include_directories(), &expected_include_directories);

        let expected_file_names = [
            FileEntry {
                path_name: AttributeValue::String(EndianSlice::new(b"foo.rs", LittleEndian)),
                directory_index: 0,
                timestamp: 0,
                size: 0,
                md5: [0; 16],
                source: None,
            },
            FileEntry {
                path_name: AttributeValue::String(EndianSlice::new(b"bar.h", LittleEndian)),
                directory_index: 1,
                timestamp: 0,
                size: 0,
                md5: [0; 16],
                source: None,
            },
        ];
        assert_eq!(header.file_names(), &expected_file_names);
    }

    #[test]
    fn test_parse_debug_line_header_length_too_short() {
        #[rustfmt::skip]
        let buf = [
            // 32-bit length = 62.
            0x3e, 0x00, 0x00, 0x00,
            // Version.
            0x04, 0x00,
            // Header length = 20. TOO SHORT!!!
            0x15, 0x00, 0x00, 0x00,
            // Minimum instruction length.
            0x01,
            // Maximum operations per byte.
            0x01,
            // Default is_stmt.
            0x01,
            // Line base.
            0x00,
            // Line range.
            0x01,
            // Opcode base.
            0x03,
            // Standard opcode lengths for opcodes 1 .. opcode base - 1.
            0x01, 0x02,
            // Include directories = '/', 'i', 'n', 'c', '\0', '/', 'i', 'n', 'c', '2', '\0', '\0'
            0x2f, 0x69, 0x6e, 0x63, 0x00, 0x2f, 0x69, 0x6e, 0x63, 0x32, 0x00, 0x00,
            // File names
                // foo.rs
                0x66, 0x6f, 0x6f, 0x2e, 0x72, 0x73, 0x00,
                0x00,
                0x00,
                0x00,
                // bar.h
                0x62, 0x61, 0x72, 0x2e, 0x68, 0x00,
                0x01,
                0x00,
                0x00,
            // End file names.
            0x00,

            // Dummy line program data.
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,

            // Dummy next line program.
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];

        let input = &mut EndianSlice::new(&buf, LittleEndian);

        match LineProgramHeader::parse(input, DebugLineOffset(0), 4, None, None) {
            Err(Error::UnexpectedEof(_)) => {}
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        }
    }

    #[test]
    fn test_parse_debug_line_unit_length_too_short() {
        #[rustfmt::skip]
        let buf = [
            // 32-bit length = 40. TOO SHORT!!!
            0x28, 0x00, 0x00, 0x00,
            // Version.
            0x04, 0x00,
            // Header length = 40.
            0x28, 0x00, 0x00, 0x00,
            // Minimum instruction length.
            0x01,
            // Maximum operations per byte.
            0x01,
            // Default is_stmt.
            0x01,
            // Line base.
            0x00,
            // Line range.
            0x01,
            // Opcode base.
            0x03,
            // Standard opcode lengths for opcodes 1 .. opcode base - 1.
            0x01, 0x02,
            // Include directories = '/', 'i', 'n', 'c', '\0', '/', 'i', 'n', 'c', '2', '\0', '\0'
            0x2f, 0x69, 0x6e, 0x63, 0x00, 0x2f, 0x69, 0x6e, 0x63, 0x32, 0x00, 0x00,
            // File names
                // foo.rs
                0x66, 0x6f, 0x6f, 0x2e, 0x72, 0x73, 0x00,
                0x00,
                0x00,
                0x00,
                // bar.h
                0x62, 0x61, 0x72, 0x2e, 0x68, 0x00,
                0x01,
                0x00,
                0x00,
            // End file names.
            0x00,

            // Dummy line program data.
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,

            // Dummy next line program.
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];

        let input = &mut EndianSlice::new(&buf, LittleEndian);

        match LineProgramHeader::parse(input, DebugLineOffset(0), 4, None, None) {
            Err(Error::UnexpectedEof(_)) => {}
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        }
    }

    const OPCODE_BASE: u8 = 13;
    const STANDARD_OPCODE_LENGTHS: &[u8] = &[0, 1, 1, 1, 1, 0, 0, 0, 1, 0, 0, 1];

    fn make_test_header(
        buf: EndianSlice<'_, LittleEndian>,
    ) -> LineProgramHeader<EndianSlice<'_, LittleEndian>> {
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 4,
            address_size: 8,
        };
        let line_encoding = LineEncoding {
            line_base: -3,
            line_range: 12,
            ..Default::default()
        };
        LineProgramHeader {
            encoding,
            offset: DebugLineOffset(0),
            unit_length: 1,
            header_length: 1,
            line_encoding,
            opcode_base: OPCODE_BASE,
            standard_opcode_lengths: EndianSlice::new(STANDARD_OPCODE_LENGTHS, LittleEndian),
            file_names: vec![
                FileEntry {
                    path_name: AttributeValue::String(EndianSlice::new(b"foo.c", LittleEndian)),
                    directory_index: 0,
                    timestamp: 0,
                    size: 0,
                    md5: [0; 16],
                    source: None,
                },
                FileEntry {
                    path_name: AttributeValue::String(EndianSlice::new(b"bar.rs", LittleEndian)),
                    directory_index: 0,
                    timestamp: 0,
                    size: 0,
                    md5: [0; 16],
                    source: None,
                },
            ],
            include_directories: vec![],
            directory_entry_format: vec![],
            file_name_entry_format: vec![],
            program_buf: buf,
            comp_dir: None,
            comp_file: None,
        }
    }

    fn make_test_program(
        buf: EndianSlice<'_, LittleEndian>,
    ) -> IncompleteLineProgram<EndianSlice<'_, LittleEndian>> {
        IncompleteLineProgram {
            header: make_test_header(buf),
        }
    }

    #[test]
    fn test_parse_special_opcodes() {
        for i in OPCODE_BASE..u8::MAX {
            let input = [i, 0, 0, 0];
            let input = EndianSlice::new(&input, LittleEndian);
            let header = make_test_header(input);

            let mut rest = input;
            let opcode =
                LineInstruction::parse(&header, &mut rest).expect("Should parse the opcode OK");

            assert_eq!(*rest, *input.range_from(1..));
            assert_eq!(opcode, LineInstruction::Special(i));
        }
    }

    #[test]
    fn test_parse_standard_opcodes() {
        fn test<Operands>(
            raw: constants::DwLns,
            operands: Operands,
            expected: LineInstruction<EndianSlice<'_, LittleEndian>>,
        ) where
            Operands: AsRef<[u8]>,
        {
            let mut input = Vec::new();
            input.push(raw.0);
            input.extend_from_slice(operands.as_ref());

            let expected_rest = [0, 1, 2, 3, 4];
            input.extend_from_slice(&expected_rest);

            let input = EndianSlice::new(&input, LittleEndian);
            let header = make_test_header(input);

            let mut rest = input;
            let opcode =
                LineInstruction::parse(&header, &mut rest).expect("Should parse the opcode OK");

            assert_eq!(opcode, expected);
            assert_eq!(*rest, expected_rest);
        }

        test(constants::DW_LNS_copy, [], LineInstruction::Copy);
        test(
            constants::DW_LNS_advance_pc,
            [42],
            LineInstruction::AdvancePc(42),
        );
        test(
            constants::DW_LNS_advance_line,
            [9],
            LineInstruction::AdvanceLine(9),
        );
        test(constants::DW_LNS_set_file, [7], LineInstruction::SetFile(7));
        test(
            constants::DW_LNS_set_column,
            [1],
            LineInstruction::SetColumn(1),
        );
        test(
            constants::DW_LNS_negate_stmt,
            [],
            LineInstruction::NegateStatement,
        );
        test(
            constants::DW_LNS_set_basic_block,
            [],
            LineInstruction::SetBasicBlock,
        );
        test(
            constants::DW_LNS_const_add_pc,
            [],
            LineInstruction::ConstAddPc,
        );
        test(
            constants::DW_LNS_fixed_advance_pc,
            [42, 0],
            LineInstruction::FixedAddPc(42),
        );
        test(
            constants::DW_LNS_set_prologue_end,
            [],
            LineInstruction::SetPrologueEnd,
        );
        test(
            constants::DW_LNS_set_isa,
            [57 + 0x80, 100],
            LineInstruction::SetIsa(12857),
        );
    }

    #[test]
    fn test_parse_unknown_standard_opcode_no_args() {
        let input = [OPCODE_BASE, 1, 2, 3];
        let input = EndianSlice::new(&input, LittleEndian);
        let mut standard_opcode_lengths = Vec::new();
        let mut header = make_test_header(input);
        standard_opcode_lengths.extend(header.standard_opcode_lengths.slice());
        standard_opcode_lengths.push(0);
        header.opcode_base += 1;
        header.standard_opcode_lengths = EndianSlice::new(&standard_opcode_lengths, LittleEndian);

        let mut rest = input;
        let opcode =
            LineInstruction::parse(&header, &mut rest).expect("Should parse the opcode OK");

        assert_eq!(
            opcode,
            LineInstruction::UnknownStandard0(constants::DwLns(OPCODE_BASE))
        );
        assert_eq!(*rest, *input.range_from(1..));
    }

    #[test]
    fn test_parse_unknown_standard_opcode_one_arg() {
        let input = [OPCODE_BASE, 1, 2, 3];
        let input = EndianSlice::new(&input, LittleEndian);
        let mut standard_opcode_lengths = Vec::new();
        let mut header = make_test_header(input);
        standard_opcode_lengths.extend(header.standard_opcode_lengths.slice());
        standard_opcode_lengths.push(1);
        header.opcode_base += 1;
        header.standard_opcode_lengths = EndianSlice::new(&standard_opcode_lengths, LittleEndian);

        let mut rest = input;
        let opcode =
            LineInstruction::parse(&header, &mut rest).expect("Should parse the opcode OK");

        assert_eq!(
            opcode,
            LineInstruction::UnknownStandard1(constants::DwLns(OPCODE_BASE), 1)
        );
        assert_eq!(*rest, *input.range_from(2..));
    }

    #[test]
    fn test_parse_unknown_standard_opcode_many_args() {
        let input = [OPCODE_BASE, 1, 2, 3];
        let input = EndianSlice::new(&input, LittleEndian);
        let args = input.range_from(1..);
        let mut standard_opcode_lengths = Vec::new();
        let mut header = make_test_header(input);
        standard_opcode_lengths.extend(header.standard_opcode_lengths.slice());
        standard_opcode_lengths.push(3);
        header.opcode_base += 1;
        header.standard_opcode_lengths = EndianSlice::new(&standard_opcode_lengths, LittleEndian);

        let mut rest = input;
        let opcode =
            LineInstruction::parse(&header, &mut rest).expect("Should parse the opcode OK");

        assert_eq!(
            opcode,
            LineInstruction::UnknownStandardN(constants::DwLns(OPCODE_BASE), args)
        );
        assert_eq!(*rest, []);
    }

    #[test]
    fn test_parse_extended_opcodes() {
        fn test<Operands>(
            raw: constants::DwLne,
            operands: Operands,
            expected: LineInstruction<EndianSlice<'_, LittleEndian>>,
        ) where
            Operands: AsRef<[u8]>,
        {
            let mut input = Vec::new();
            input.push(0);

            let operands = operands.as_ref();
            input.push(1 + operands.len() as u8);

            input.push(raw.0);
            input.extend_from_slice(operands);

            let expected_rest = [0, 1, 2, 3, 4];
            input.extend_from_slice(&expected_rest);

            let input = EndianSlice::new(&input, LittleEndian);
            let header = make_test_header(input);

            let mut rest = input;
            let opcode =
                LineInstruction::parse(&header, &mut rest).expect("Should parse the opcode OK");

            assert_eq!(opcode, expected);
            assert_eq!(*rest, expected_rest);
        }

        test(
            constants::DW_LNE_end_sequence,
            [],
            LineInstruction::EndSequence,
        );
        test(
            constants::DW_LNE_set_address,
            [1, 2, 3, 4, 5, 6, 7, 8],
            LineInstruction::SetAddress(578_437_695_752_307_201),
        );
        test(
            constants::DW_LNE_set_discriminator,
            [42],
            LineInstruction::SetDiscriminator(42),
        );

        let mut file = Vec::new();
        // "foo.c"
        let path_name = [b'f', b'o', b'o', b'.', b'c', 0];
        file.extend_from_slice(&path_name);
        // Directory index.
        file.push(0);
        // Last modification of file.
        file.push(1);
        // Size of file.
        file.push(2);

        test(
            constants::DW_LNE_define_file,
            file,
            LineInstruction::DefineFile(FileEntry {
                path_name: AttributeValue::String(EndianSlice::new(b"foo.c", LittleEndian)),
                directory_index: 0,
                timestamp: 1,
                size: 2,
                md5: [0; 16],
                source: None,
            }),
        );

        // Unknown extended opcode.
        let operands = [1, 2, 3, 4, 5, 6];
        let opcode = constants::DwLne(99);
        test(
            opcode,
            operands,
            LineInstruction::UnknownExtended(opcode, EndianSlice::new(&operands, LittleEndian)),
        );
    }

    #[test]
    fn test_file_entry_directory() {
        let path_name = [b'f', b'o', b'o', b'.', b'r', b's', 0];

        let mut file = FileEntry {
            path_name: AttributeValue::String(EndianSlice::new(&path_name, LittleEndian)),
            directory_index: 1,
            timestamp: 0,
            size: 0,
            md5: [0; 16],
            source: None,
        };

        let mut header = make_test_header(EndianSlice::new(&[], LittleEndian));

        let dir = AttributeValue::String(EndianSlice::new(b"dir", LittleEndian));
        header.include_directories.push(dir);

        assert_eq!(file.directory(&header), Some(dir));

        // Now test the compilation's current directory.
        file.directory_index = 0;
        assert_eq!(file.directory(&header), None);
    }

    fn assert_exec_opcode<'input>(
        header: LineProgramHeader<EndianSlice<'input, LittleEndian>>,
        mut registers: LineRow,
        opcode: LineInstruction<EndianSlice<'input, LittleEndian>>,
        expected_registers: LineRow,
        expect_new_row: bool,
    ) {
        let mut program = IncompleteLineProgram { header };
        let is_new_row = registers.execute(opcode, &mut program);

        assert_eq!(is_new_row, Ok(expect_new_row));
        assert_eq!(registers, expected_registers);
    }

    #[test]
    fn test_exec_special_noop() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));

        let initial_registers = LineRow::new(&header);
        let opcode = LineInstruction::Special(16);
        let expected_registers = initial_registers;

        assert_exec_opcode(header, initial_registers, opcode, expected_registers, true);
    }

    #[test]
    fn test_exec_special_negative_line_advance() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));

        let mut initial_registers = LineRow::new(&header);
        initial_registers.line.0 = 10;

        let opcode = LineInstruction::Special(13);

        let mut expected_registers = initial_registers;
        expected_registers.line.0 -= 3;

        assert_exec_opcode(header, initial_registers, opcode, expected_registers, true);
    }

    #[test]
    fn test_exec_special_positive_line_advance() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));

        let initial_registers = LineRow::new(&header);

        let opcode = LineInstruction::Special(19);

        let mut expected_registers = initial_registers;
        expected_registers.line.0 += 3;

        assert_exec_opcode(header, initial_registers, opcode, expected_registers, true);
    }

    #[test]
    fn test_exec_special_positive_address_advance() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));

        let initial_registers = LineRow::new(&header);

        let opcode = LineInstruction::Special(52);

        let mut expected_registers = initial_registers;
        expected_registers.address += 3;

        assert_exec_opcode(header, initial_registers, opcode, expected_registers, true);
    }

    #[test]
    fn test_exec_special_positive_address_and_line_advance() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));

        let initial_registers = LineRow::new(&header);

        let opcode = LineInstruction::Special(55);

        let mut expected_registers = initial_registers;
        expected_registers.address += 3;
        expected_registers.line.0 += 3;

        assert_exec_opcode(header, initial_registers, opcode, expected_registers, true);
    }

    #[test]
    fn test_exec_special_positive_address_and_negative_line_advance() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));

        let mut initial_registers = LineRow::new(&header);
        initial_registers.line.0 = 10;

        let opcode = LineInstruction::Special(49);

        let mut expected_registers = initial_registers;
        expected_registers.address += 3;
        expected_registers.line.0 -= 3;

        assert_exec_opcode(header, initial_registers, opcode, expected_registers, true);
    }

    #[test]
    fn test_exec_special_line_underflow() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));

        let mut initial_registers = LineRow::new(&header);
        initial_registers.line.0 = 2;

        // -3 line advance.
        let opcode = LineInstruction::Special(13);

        let mut expected_registers = initial_registers;
        // Clamp at 0. No idea if this is the best way to handle this situation
        // or not...
        expected_registers.line.0 = 0;

        assert_exec_opcode(header, initial_registers, opcode, expected_registers, true);
    }

    #[test]
    fn test_exec_copy() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));

        let mut initial_registers = LineRow::new(&header);
        initial_registers.address = 1337;
        initial_registers.line.0 = 42;

        let opcode = LineInstruction::Copy;

        let expected_registers = initial_registers;

        assert_exec_opcode(header, initial_registers, opcode, expected_registers, true);
    }

    #[test]
    fn test_exec_advance_pc() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));
        let initial_registers = LineRow::new(&header);
        let opcode = LineInstruction::AdvancePc(42);

        let mut expected_registers = initial_registers;
        expected_registers.address += 42;

        assert_exec_opcode(header, initial_registers, opcode, expected_registers, false);
    }

    #[test]
    fn test_exec_advance_pc_overflow_32() {
        let mut header = make_test_header(EndianSlice::new(&[], LittleEndian));
        header.encoding.address_size = 4;
        let mut registers = LineRow::new(&header);
        registers.address = u32::MAX.into();
        let opcode = LineInstruction::AdvancePc(42);
        let mut program = IncompleteLineProgram { header };
        let result = registers.execute(opcode, &mut program);
        assert_eq!(result, Err(Error::AddressOverflow));
    }

    #[test]
    fn test_exec_advance_pc_overflow_64() {
        let mut header = make_test_header(EndianSlice::new(&[], LittleEndian));
        header.encoding.address_size = 8;
        let mut registers = LineRow::new(&header);
        registers.address = u64::MAX;
        let opcode = LineInstruction::AdvancePc(42);
        let mut program = IncompleteLineProgram { header };
        let result = registers.execute(opcode, &mut program);
        assert_eq!(result, Err(Error::AddressOverflow));
    }

    #[test]
    fn test_exec_advance_line() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));
        let initial_registers = LineRow::new(&header);
        let opcode = LineInstruction::AdvanceLine(42);

        let mut expected_registers = initial_registers;
        expected_registers.line.0 += 42;

        assert_exec_opcode(header, initial_registers, opcode, expected_registers, false);
    }

    #[test]
    fn test_exec_advance_line_overflow() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));
        let opcode = LineInstruction::AdvanceLine(42);

        let mut initial_registers = LineRow::new(&header);
        initial_registers.line.0 = u64::MAX;

        let mut expected_registers = initial_registers;
        expected_registers.line.0 = 41;

        assert_exec_opcode(header, initial_registers, opcode, expected_registers, false);
    }

    #[test]
    fn test_exec_set_file_in_bounds() {
        for file_idx in 1..3 {
            let header = make_test_header(EndianSlice::new(&[], LittleEndian));
            let initial_registers = LineRow::new(&header);
            let opcode = LineInstruction::SetFile(file_idx);

            let mut expected_registers = initial_registers;
            expected_registers.file = file_idx;

            assert_exec_opcode(header, initial_registers, opcode, expected_registers, false);
        }
    }

    #[test]
    fn test_exec_set_file_out_of_bounds() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));
        let initial_registers = LineRow::new(&header);
        let opcode = LineInstruction::SetFile(100);

        // The spec doesn't say anything about rejecting input programs
        // that set the file register out of bounds of the actual number
        // of files that have been defined. Instead, we cross our
        // fingers and hope that one gets defined before
        // `LineRow::file` gets called and handle the error at
        // that time if need be.
        let mut expected_registers = initial_registers;
        expected_registers.file = 100;

        assert_exec_opcode(header, initial_registers, opcode, expected_registers, false);
    }

    #[test]
    fn test_file_entry_file_index_out_of_bounds() {
        // These indices are 1-based, so 0 is invalid. 100 is way more than the
        // number of files defined in the header.
        let out_of_bounds_indices = [0, 100];

        for file_idx in &out_of_bounds_indices[..] {
            let header = make_test_header(EndianSlice::new(&[], LittleEndian));
            let mut row = LineRow::new(&header);

            row.file = *file_idx;

            assert_eq!(row.file(&header), None);
        }
    }

    #[test]
    fn test_file_entry_file_index_in_bounds() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));
        let mut row = LineRow::new(&header);

        row.file = 2;

        assert_eq!(row.file(&header), Some(&header.file_names()[1]));
    }

    #[test]
    fn test_exec_set_column() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));
        let initial_registers = LineRow::new(&header);
        let opcode = LineInstruction::SetColumn(42);

        let mut expected_registers = initial_registers;
        expected_registers.column = 42;

        assert_exec_opcode(header, initial_registers, opcode, expected_registers, false);
    }

    #[test]
    fn test_exec_negate_statement() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));
        let initial_registers = LineRow::new(&header);
        let opcode = LineInstruction::NegateStatement;

        let mut expected_registers = initial_registers;
        expected_registers.is_stmt = !initial_registers.is_stmt;

        assert_exec_opcode(header, initial_registers, opcode, expected_registers, false);
    }

    #[test]
    fn test_exec_set_basic_block() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));

        let mut initial_registers = LineRow::new(&header);
        initial_registers.basic_block = false;

        let opcode = LineInstruction::SetBasicBlock;

        let mut expected_registers = initial_registers;
        expected_registers.basic_block = true;

        assert_exec_opcode(header, initial_registers, opcode, expected_registers, false);
    }

    #[test]
    fn test_exec_const_add_pc() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));
        let initial_registers = LineRow::new(&header);
        let opcode = LineInstruction::ConstAddPc;

        let mut expected_registers = initial_registers;
        expected_registers.address += 20;

        assert_exec_opcode(header, initial_registers, opcode, expected_registers, false);
    }

    #[test]
    fn test_exec_const_add_pc_overflow() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));
        let mut registers = LineRow::new(&header);
        registers.address = u64::MAX;
        let opcode = LineInstruction::ConstAddPc;
        let mut program = IncompleteLineProgram { header };
        let result = registers.execute(opcode, &mut program);
        assert_eq!(result, Err(Error::AddressOverflow));
    }

    #[test]
    fn test_exec_fixed_add_pc() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));

        let mut initial_registers = LineRow::new(&header);
        initial_registers.op_index.0 = 1;

        let opcode = LineInstruction::FixedAddPc(10);

        let mut expected_registers = initial_registers;
        expected_registers.address += 10;
        expected_registers.op_index.0 = 0;

        assert_exec_opcode(header, initial_registers, opcode, expected_registers, false);
    }

    #[test]
    fn test_exec_fixed_add_pc_overflow() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));
        let mut registers = LineRow::new(&header);
        registers.address = u64::MAX;
        registers.op_index.0 = 1;
        let opcode = LineInstruction::FixedAddPc(10);
        let mut program = IncompleteLineProgram { header };
        let result = registers.execute(opcode, &mut program);
        assert_eq!(result, Err(Error::AddressOverflow));
    }

    #[test]
    fn test_exec_set_prologue_end() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));

        let mut initial_registers = LineRow::new(&header);
        initial_registers.prologue_end = false;

        let opcode = LineInstruction::SetPrologueEnd;

        let mut expected_registers = initial_registers;
        expected_registers.prologue_end = true;

        assert_exec_opcode(header, initial_registers, opcode, expected_registers, false);
    }

    #[test]
    fn test_exec_set_isa() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));
        let initial_registers = LineRow::new(&header);
        let opcode = LineInstruction::SetIsa(1993);

        let mut expected_registers = initial_registers;
        expected_registers.isa = 1993;

        assert_exec_opcode(header, initial_registers, opcode, expected_registers, false);
    }

    #[test]
    fn test_exec_unknown_standard_0() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));
        let initial_registers = LineRow::new(&header);
        let opcode = LineInstruction::UnknownStandard0(constants::DwLns(111));
        let expected_registers = initial_registers;
        assert_exec_opcode(header, initial_registers, opcode, expected_registers, false);
    }

    #[test]
    fn test_exec_unknown_standard_1() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));
        let initial_registers = LineRow::new(&header);
        let opcode = LineInstruction::UnknownStandard1(constants::DwLns(111), 2);
        let expected_registers = initial_registers;
        assert_exec_opcode(header, initial_registers, opcode, expected_registers, false);
    }

    #[test]
    fn test_exec_unknown_standard_n() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));
        let initial_registers = LineRow::new(&header);
        let opcode = LineInstruction::UnknownStandardN(
            constants::DwLns(111),
            EndianSlice::new(&[2, 2, 2], LittleEndian),
        );
        let expected_registers = initial_registers;
        assert_exec_opcode(header, initial_registers, opcode, expected_registers, false);
    }

    #[test]
    fn test_exec_end_sequence() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));
        let initial_registers = LineRow::new(&header);
        let opcode = LineInstruction::EndSequence;

        let mut expected_registers = initial_registers;
        expected_registers.end_sequence = true;

        assert_exec_opcode(header, initial_registers, opcode, expected_registers, true);
    }

    #[test]
    fn test_exec_set_address() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));
        let initial_registers = LineRow::new(&header);
        let opcode = LineInstruction::SetAddress(3030);

        let mut expected_registers = initial_registers;
        expected_registers.address = 3030;

        assert_exec_opcode(header, initial_registers, opcode, expected_registers, false);
    }

    #[test]
    fn test_exec_set_address_tombstone() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));
        let initial_registers = LineRow::new(&header);
        let opcode = LineInstruction::SetAddress(!0);

        let mut expected_registers = initial_registers;
        expected_registers.tombstone = true;

        assert_exec_opcode(header, initial_registers, opcode, expected_registers, false);
    }

    #[test]
    fn test_exec_set_address_backwards() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));
        let mut initial_registers = LineRow::new(&header);
        initial_registers.address = 1;
        let opcode = LineInstruction::SetAddress(0);

        let mut expected_registers = initial_registers;
        expected_registers.tombstone = true;

        assert_exec_opcode(header, initial_registers, opcode, expected_registers, false);
    }

    #[test]
    fn test_exec_define_file() {
        let mut program = make_test_program(EndianSlice::new(&[], LittleEndian));
        let mut row = LineRow::new(program.header());

        let file = FileEntry {
            path_name: AttributeValue::String(EndianSlice::new(b"test.cpp", LittleEndian)),
            directory_index: 0,
            timestamp: 0,
            size: 0,
            md5: [0; 16],
            source: None,
        };

        let opcode = LineInstruction::DefineFile(file);
        let is_new_row = row.execute(opcode, &mut program).unwrap();

        assert!(!is_new_row);
        assert_eq!(Some(&file), program.header().file_names.last());
    }

    #[test]
    fn test_exec_set_discriminator() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));
        let initial_registers = LineRow::new(&header);
        let opcode = LineInstruction::SetDiscriminator(9);

        let mut expected_registers = initial_registers;
        expected_registers.discriminator = 9;

        assert_exec_opcode(header, initial_registers, opcode, expected_registers, false);
    }

    #[test]
    fn test_exec_unknown_extended() {
        let header = make_test_header(EndianSlice::new(&[], LittleEndian));
        let initial_registers = LineRow::new(&header);
        let opcode = LineInstruction::UnknownExtended(
            constants::DwLne(74),
            EndianSlice::new(&[], LittleEndian),
        );
        let expected_registers = initial_registers;
        assert_exec_opcode(header, initial_registers, opcode, expected_registers, false);
    }

    /// Ensure that `LineRows<R,P>` is covariant wrt R.
    /// This only needs to compile.
    #[allow(dead_code, unreachable_code, unused_variables)]
    #[allow(clippy::diverging_sub_expression)]
    fn test_line_rows_variance<'a, 'b>(_: &'a [u8], _: &'b [u8])
    where
        'a: 'b,
    {
        let a: &OneShotLineRows<EndianSlice<'a, LittleEndian>> = unimplemented!();
        let _: &OneShotLineRows<EndianSlice<'b, LittleEndian>> = a;
    }

    #[test]
    fn test_parse_debug_line_v5_ok() {
        let expected_lengths = &[1, 2];
        let expected_program = &[0, 1, 2, 3, 4];
        let expected_rest = &[5, 6, 7, 8, 9];
        let expected_include_directories = [
            AttributeValue::String(EndianSlice::new(b"dir1", LittleEndian)),
            AttributeValue::String(EndianSlice::new(b"dir2", LittleEndian)),
        ];
        let expected_file_names = [
            FileEntry {
                path_name: AttributeValue::String(EndianSlice::new(b"file1", LittleEndian)),
                directory_index: 0,
                timestamp: 0,
                size: 0,
                md5: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
                source: Some(AttributeValue::String(EndianSlice::new(
                    b"foobar",
                    LittleEndian,
                ))),
            },
            FileEntry {
                path_name: AttributeValue::String(EndianSlice::new(b"file2", LittleEndian)),
                directory_index: 1,
                timestamp: 0,
                size: 0,
                md5: [
                    11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
                ],
                source: Some(AttributeValue::String(EndianSlice::new(
                    b"quux",
                    LittleEndian,
                ))),
            },
        ];

        for format in [Format::Dwarf32, Format::Dwarf64] {
            let length = Label::new();
            let header_length = Label::new();
            let start = Label::new();
            let header_start = Label::new();
            let end = Label::new();
            let header_end = Label::new();
            let section = Section::with_endian(Endian::Little)
                .initial_length(format, &length, &start)
                .D16(5)
                // Address size.
                .D8(4)
                // Segment selector size.
                .D8(0)
                .word_label(format.word_size(), &header_length)
                .mark(&header_start)
                // Minimum instruction length.
                .D8(1)
                // Maximum operations per byte.
                .D8(1)
                // Default is_stmt.
                .D8(1)
                // Line base.
                .D8(0)
                // Line range.
                .D8(1)
                // Opcode base.
                .D8(expected_lengths.len() as u8 + 1)
                // Standard opcode lengths for opcodes 1 .. opcode base - 1.
                .append_bytes(expected_lengths)
                // Directory entry format count.
                .D8(1)
                .uleb(constants::DW_LNCT_path.0 as u64)
                .uleb(constants::DW_FORM_string.0 as u64)
                // Directory count.
                .D8(2)
                .append_bytes(b"dir1\0")
                .append_bytes(b"dir2\0")
                // File entry format count.
                .D8(4)
                .uleb(constants::DW_LNCT_path.0 as u64)
                .uleb(constants::DW_FORM_string.0 as u64)
                .uleb(constants::DW_LNCT_directory_index.0 as u64)
                .uleb(constants::DW_FORM_data1.0 as u64)
                .uleb(constants::DW_LNCT_MD5.0 as u64)
                .uleb(constants::DW_FORM_data16.0 as u64)
                .uleb(constants::DW_LNCT_LLVM_source.0 as u64)
                .uleb(constants::DW_FORM_string.0 as u64)
                // File count.
                .D8(2)
                .append_bytes(b"file1\0")
                .D8(0)
                .append_bytes(&expected_file_names[0].md5)
                .append_bytes(b"foobar\0")
                .append_bytes(b"file2\0")
                .D8(1)
                .append_bytes(&expected_file_names[1].md5)
                .append_bytes(b"quux\0")
                .mark(&header_end)
                // Dummy line program data.
                .append_bytes(expected_program)
                .mark(&end)
                // Dummy trailing data.
                .append_bytes(expected_rest);
            length.set_const((&end - &start) as u64);
            header_length.set_const((&header_end - &header_start) as u64);
            let section = section.get_contents().unwrap();

            let input = &mut EndianSlice::new(&section, LittleEndian);

            let header = LineProgramHeader::parse(input, DebugLineOffset(0), 0, None, None)
                .expect("should parse header ok");

            assert_eq!(header.raw_program_buf().slice(), expected_program);
            assert_eq!(input.slice(), expected_rest);

            assert_eq!(header.offset, DebugLineOffset(0));
            assert_eq!(header.version(), 5);
            assert_eq!(header.address_size(), 4);
            assert_eq!(header.minimum_instruction_length(), 1);
            assert_eq!(header.maximum_operations_per_instruction(), 1);
            assert!(header.default_is_stmt());
            assert_eq!(header.line_base(), 0);
            assert_eq!(header.line_range(), 1);
            assert_eq!(header.opcode_base(), expected_lengths.len() as u8 + 1);
            assert_eq!(header.standard_opcode_lengths().slice(), expected_lengths);
            assert_eq!(
                header.directory_entry_format(),
                &[FileEntryFormat {
                    content_type: constants::DW_LNCT_path,
                    form: constants::DW_FORM_string,
                }]
            );
            assert_eq!(header.include_directories(), expected_include_directories);
            assert_eq!(header.directory(0), Some(expected_include_directories[0]));
            assert_eq!(
                header.file_name_entry_format(),
                &[
                    FileEntryFormat {
                        content_type: constants::DW_LNCT_path,
                        form: constants::DW_FORM_string,
                    },
                    FileEntryFormat {
                        content_type: constants::DW_LNCT_directory_index,
                        form: constants::DW_FORM_data1,
                    },
                    FileEntryFormat {
                        content_type: constants::DW_LNCT_MD5,
                        form: constants::DW_FORM_data16,
                    },
                    FileEntryFormat {
                        content_type: constants::DW_LNCT_LLVM_source,
                        form: constants::DW_FORM_string,
                    }
                ]
            );
            assert_eq!(header.file_names(), expected_file_names);
            assert_eq!(header.file(0), Some(&expected_file_names[0]));
        }
    }

    #[test]
    fn test_sequences() {
        #[rustfmt::skip]
        let buf = [
            // 32-bit length
            94, 0x00, 0x00, 0x00,
            // Version.
            0x04, 0x00,
            // Header length = 40.
            0x28, 0x00, 0x00, 0x00,
            // Minimum instruction length.
            0x01,
            // Maximum operations per byte.
            0x01,
            // Default is_stmt.
            0x01,
            // Line base.
            0x00,
            // Line range.
            0x01,
            // Opcode base.
            0x03,
            // Standard opcode lengths for opcodes 1 .. opcode base - 1.
            0x01, 0x02,
            // Include directories = '/', 'i', 'n', 'c', '\0', '/', 'i', 'n', 'c', '2', '\0', '\0'
            0x2f, 0x69, 0x6e, 0x63, 0x00, 0x2f, 0x69, 0x6e, 0x63, 0x32, 0x00, 0x00,
            // File names
                // foo.rs
                0x66, 0x6f, 0x6f, 0x2e, 0x72, 0x73, 0x00,
                0x00,
                0x00,
                0x00,
                // bar.h
                0x62, 0x61, 0x72, 0x2e, 0x68, 0x00,
                0x01,
                0x00,
                0x00,
            // End file names.
            0x00,

            0, 5, constants::DW_LNE_set_address.0, 1, 0, 0, 0,
            constants::DW_LNS_copy.0,
            constants::DW_LNS_advance_pc.0, 1,
            constants::DW_LNS_copy.0,
            constants::DW_LNS_advance_pc.0, 2,
            0, 1, constants::DW_LNE_end_sequence.0,

            // Tombstone
            0, 5, constants::DW_LNE_set_address.0, 0xff, 0xff, 0xff, 0xff,
            constants::DW_LNS_copy.0,
            constants::DW_LNS_advance_pc.0, 1,
            constants::DW_LNS_copy.0,
            constants::DW_LNS_advance_pc.0, 2,
            0, 1, constants::DW_LNE_end_sequence.0,

            0, 5, constants::DW_LNE_set_address.0, 11, 0, 0, 0,
            constants::DW_LNS_copy.0,
            constants::DW_LNS_advance_pc.0, 1,
            constants::DW_LNS_copy.0,
            constants::DW_LNS_advance_pc.0, 2,
            0, 1, constants::DW_LNE_end_sequence.0,
        ];
        assert_eq!(buf[0] as usize, buf.len() - 4);

        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        let header = LineProgramHeader::parse(rest, DebugLineOffset(0), 4, None, None)
            .expect("should parse header ok");
        let program = IncompleteLineProgram { header };

        let sequences = program.sequences().unwrap().1;
        assert_eq!(sequences.len(), 2);
        assert_eq!(sequences[0].start, 1);
        assert_eq!(sequences[0].end, 4);
        assert_eq!(sequences[1].start, 11);
        assert_eq!(sequences[1].end, 14);
    }
}
