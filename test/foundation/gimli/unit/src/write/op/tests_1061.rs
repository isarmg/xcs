#[cfg(test)]
#[cfg(feature = "read")]
mod tests {
    use super::*;
    use crate::common::{DebugInfoOffset, Format};
    use crate::read;
    use crate::write::{AttributeValue, Dwarf, EndianVec, LineProgram, Sections, Unit};
    use crate::LittleEndian;
    use std::collections::HashMap;

    #[test]
    #[allow(clippy::type_complexity)]
    fn test_operation() {
        for version in [2, 3, 4, 5] {
            for address_size in [4, 8] {
                for format in [Format::Dwarf32, Format::Dwarf64] {
                    let encoding = Encoding {
                        format,
                        version,
                        address_size,
                    };

                    let mut dwarf = Dwarf::new();
                    let unit_id = dwarf.units.add(Unit::new(encoding, LineProgram::none()));
                    let unit = dwarf.units.get_mut(unit_id);

                    // Create an entry that can be referenced by the expression.
                    let entry_id = unit.add(unit.root(), constants::DW_TAG_base_type);
                    let reference = Reference::Entry(unit_id, entry_id);

                    // The offsets for the above entry when reading back the expression.
                    struct ReadState {
                        debug_info_offset: DebugInfoOffset,
                        entry_offset: read::UnitOffset,
                    }

                    let mut reg_expression = Expression::new();
                    reg_expression.op_reg(Register(23));

                    let operations: &[(
                        &dyn Fn(&mut Expression),
                        Operation,
                        &dyn Fn(&ReadState) -> read::Operation<_>,
                    )] = &[
                        (
                            &|x| x.op_deref(),
                            Operation::Deref { space: false },
                            &|_| read::Operation::Deref {
                                base_type: read::UnitOffset(0),
                                size: address_size,
                                space: false,
                            },
                        ),
                        (
                            &|x| x.op_xderef(),
                            Operation::Deref { space: true },
                            &|_| read::Operation::Deref {
                                base_type: read::UnitOffset(0),
                                size: address_size,
                                space: true,
                            },
                        ),
                        (
                            &|x| x.op_deref_size(2),
                            Operation::DerefSize {
                                space: false,
                                size: 2,
                            },
                            &|_| read::Operation::Deref {
                                base_type: read::UnitOffset(0),
                                size: 2,
                                space: false,
                            },
                        ),
                        (
                            &|x| x.op_xderef_size(2),
                            Operation::DerefSize {
                                space: true,
                                size: 2,
                            },
                            &|_| read::Operation::Deref {
                                base_type: read::UnitOffset(0),
                                size: 2,
                                space: true,
                            },
                        ),
                        (
                            &|x| x.op_deref_type(2, entry_id),
                            Operation::DerefType {
                                space: false,
                                size: 2,
                                base: entry_id,
                            },
                            &|x| read::Operation::Deref {
                                base_type: x.entry_offset,
                                size: 2,
                                space: false,
                            },
                        ),
                        (
                            &|x| x.op_xderef_type(2, entry_id),
                            Operation::DerefType {
                                space: true,
                                size: 2,
                                base: entry_id,
                            },
                            &|x| read::Operation::Deref {
                                base_type: x.entry_offset,
                                size: 2,
                                space: true,
                            },
                        ),
                        (
                            &|x| x.op(constants::DW_OP_drop),
                            Operation::Simple(constants::DW_OP_drop),
                            &|_| read::Operation::Drop,
                        ),
                        (&|x| x.op_pick(0), Operation::Pick(0), &|_| {
                            read::Operation::Pick { index: 0 }
                        }),
                        (&|x| x.op_pick(1), Operation::Pick(1), &|_| {
                            read::Operation::Pick { index: 1 }
                        }),
                        (&|x| x.op_pick(2), Operation::Pick(2), &|_| {
                            read::Operation::Pick { index: 2 }
                        }),
                        (
                            &|x| x.op(constants::DW_OP_swap),
                            Operation::Simple(constants::DW_OP_swap),
                            &|_| read::Operation::Swap,
                        ),
                        (
                            &|x| x.op(constants::DW_OP_rot),
                            Operation::Simple(constants::DW_OP_rot),
                            &|_| read::Operation::Rot,
                        ),
                        (
                            &|x| x.op(constants::DW_OP_abs),
                            Operation::Simple(constants::DW_OP_abs),
                            &|_| read::Operation::Abs,
                        ),
                        (
                            &|x| x.op(constants::DW_OP_and),
                            Operation::Simple(constants::DW_OP_and),
                            &|_| read::Operation::And,
                        ),
                        (
                            &|x| x.op(constants::DW_OP_div),
                            Operation::Simple(constants::DW_OP_div),
                            &|_| read::Operation::Div,
                        ),
                        (
                            &|x| x.op(constants::DW_OP_minus),
                            Operation::Simple(constants::DW_OP_minus),
                            &|_| read::Operation::Minus,
                        ),
                        (
                            &|x| x.op(constants::DW_OP_mod),
                            Operation::Simple(constants::DW_OP_mod),
                            &|_| read::Operation::Mod,
                        ),
                        (
                            &|x| x.op(constants::DW_OP_mul),
                            Operation::Simple(constants::DW_OP_mul),
                            &|_| read::Operation::Mul,
                        ),
                        (
                            &|x| x.op(constants::DW_OP_neg),
                            Operation::Simple(constants::DW_OP_neg),
                            &|_| read::Operation::Neg,
                        ),
                        (
                            &|x| x.op(constants::DW_OP_not),
                            Operation::Simple(constants::DW_OP_not),
                            &|_| read::Operation::Not,
                        ),
                        (
                            &|x| x.op(constants::DW_OP_or),
                            Operation::Simple(constants::DW_OP_or),
                            &|_| read::Operation::Or,
                        ),
                        (
                            &|x| x.op(constants::DW_OP_plus),
                            Operation::Simple(constants::DW_OP_plus),
                            &|_| read::Operation::Plus,
                        ),
                        (
                            &|x| x.op_plus_uconst(23),
                            Operation::PlusConstant(23),
                            &|_| read::Operation::PlusConstant { value: 23 },
                        ),
                        (
                            &|x| x.op(constants::DW_OP_shl),
                            Operation::Simple(constants::DW_OP_shl),
                            &|_| read::Operation::Shl,
                        ),
                        (
                            &|x| x.op(constants::DW_OP_shr),
                            Operation::Simple(constants::DW_OP_shr),
                            &|_| read::Operation::Shr,
                        ),
                        (
                            &|x| x.op(constants::DW_OP_shra),
                            Operation::Simple(constants::DW_OP_shra),
                            &|_| read::Operation::Shra,
                        ),
                        (
                            &|x| x.op(constants::DW_OP_xor),
                            Operation::Simple(constants::DW_OP_xor),
                            &|_| read::Operation::Xor,
                        ),
                        (
                            &|x| x.op(constants::DW_OP_eq),
                            Operation::Simple(constants::DW_OP_eq),
                            &|_| read::Operation::Eq,
                        ),
                        (
                            &|x| x.op(constants::DW_OP_ge),
                            Operation::Simple(constants::DW_OP_ge),
                            &|_| read::Operation::Ge,
                        ),
                        (
                            &|x| x.op(constants::DW_OP_gt),
                            Operation::Simple(constants::DW_OP_gt),
                            &|_| read::Operation::Gt,
                        ),
                        (
                            &|x| x.op(constants::DW_OP_le),
                            Operation::Simple(constants::DW_OP_le),
                            &|_| read::Operation::Le,
                        ),
                        (
                            &|x| x.op(constants::DW_OP_lt),
                            Operation::Simple(constants::DW_OP_lt),
                            &|_| read::Operation::Lt,
                        ),
                        (
                            &|x| x.op(constants::DW_OP_ne),
                            Operation::Simple(constants::DW_OP_ne),
                            &|_| read::Operation::Ne,
                        ),
                        (
                            &|x| x.op_constu(23),
                            Operation::UnsignedConstant(23),
                            &|_| read::Operation::UnsignedConstant { value: 23 },
                        ),
                        (
                            &|x| x.op_consts(-23),
                            Operation::SignedConstant(-23),
                            &|_| read::Operation::SignedConstant { value: -23 },
                        ),
                        (
                            &|x| x.op_reg(Register(23)),
                            Operation::Register(Register(23)),
                            &|_| read::Operation::Register {
                                register: Register(23),
                            },
                        ),
                        (
                            &|x| x.op_reg(Register(123)),
                            Operation::Register(Register(123)),
                            &|_| read::Operation::Register {
                                register: Register(123),
                            },
                        ),
                        (
                            &|x| x.op_breg(Register(23), 34),
                            Operation::RegisterOffset(Register(23), 34),
                            &|_| read::Operation::RegisterOffset {
                                register: Register(23),
                                offset: 34,
                                base_type: read::UnitOffset(0),
                            },
                        ),
                        (
                            &|x| x.op_breg(Register(123), 34),
                            Operation::RegisterOffset(Register(123), 34),
                            &|_| read::Operation::RegisterOffset {
                                register: Register(123),
                                offset: 34,
                                base_type: read::UnitOffset(0),
                            },
                        ),
                        (
                            &|x| x.op_regval_type(Register(23), entry_id),
                            Operation::RegisterType(Register(23), entry_id),
                            &|x| read::Operation::RegisterOffset {
                                register: Register(23),
                                offset: 0,
                                base_type: x.entry_offset,
                            },
                        ),
                        (&|x| x.op_fbreg(34), Operation::FrameOffset(34), &|_| {
                            read::Operation::FrameOffset { offset: 34 }
                        }),
                        (
                            &|x| x.op(constants::DW_OP_nop),
                            Operation::Simple(constants::DW_OP_nop),
                            &|_| read::Operation::Nop,
                        ),
                        (
                            &|x| x.op(constants::DW_OP_push_object_address),
                            Operation::Simple(constants::DW_OP_push_object_address),
                            &|_| read::Operation::PushObjectAddress,
                        ),
                        (&|x| x.op_call(entry_id), Operation::Call(entry_id), &|x| {
                            read::Operation::Call {
                                offset: read::DieReference::UnitRef(x.entry_offset),
                            }
                        }),
                        (
                            &|x| x.op_call_ref(reference),
                            Operation::CallRef(reference),
                            &|x| read::Operation::Call {
                                offset: read::DieReference::DebugInfoRef(x.debug_info_offset),
                            },
                        ),
                        (
                            &|x| x.op(constants::DW_OP_form_tls_address),
                            Operation::Simple(constants::DW_OP_form_tls_address),
                            &|_| read::Operation::TLS,
                        ),
                        (
                            &|x| x.op(constants::DW_OP_call_frame_cfa),
                            Operation::Simple(constants::DW_OP_call_frame_cfa),
                            &|_| read::Operation::CallFrameCFA,
                        ),
                        (
                            &|x| x.op_piece(23),
                            Operation::Piece { size_in_bytes: 23 },
                            &|_| read::Operation::Piece {
                                size_in_bits: 23 * 8,
                                bit_offset: None,
                            },
                        ),
                        (
                            &|x| x.op_bit_piece(23, 34),
                            Operation::BitPiece {
                                size_in_bits: 23,
                                bit_offset: 34,
                            },
                            &|_| read::Operation::Piece {
                                size_in_bits: 23,
                                bit_offset: Some(34),
                            },
                        ),
                        (
                            &|x| x.op_implicit_value(vec![23].into()),
                            Operation::ImplicitValue(vec![23].into()),
                            &|_| read::Operation::ImplicitValue {
                                data: read::EndianSlice::new(&[23], LittleEndian),
                            },
                        ),
                        (
                            &|x| x.op(constants::DW_OP_stack_value),
                            Operation::Simple(constants::DW_OP_stack_value),
                            &|_| read::Operation::StackValue,
                        ),
                        (
                            &|x| x.op_implicit_pointer(reference, 23),
                            Operation::ImplicitPointer {
                                entry: reference,
                                byte_offset: 23,
                            },
                            &|x| read::Operation::ImplicitPointer {
                                value: x.debug_info_offset,
                                byte_offset: 23,
                            },
                        ),
                        (
                            &|x| x.op_entry_value(reg_expression.clone()),
                            Operation::EntryValue(reg_expression.clone()),
                            &|_| read::Operation::EntryValue {
                                expression: read::EndianSlice::new(
                                    &[constants::DW_OP_reg23.0],
                                    LittleEndian,
                                ),
                            },
                        ),
                        (
                            &|x| x.op_gnu_parameter_ref(entry_id),
                            Operation::ParameterRef(entry_id),
                            &|x| read::Operation::ParameterRef {
                                offset: x.entry_offset,
                            },
                        ),
                        (
                            &|x| x.op_addr(Address::Constant(23)),
                            Operation::Address(Address::Constant(23)),
                            &|_| read::Operation::Address { address: 23 },
                        ),
                        (
                            &|x| x.op_const_type(entry_id, vec![23].into()),
                            Operation::ConstantType(entry_id, vec![23].into()),
                            &|x| read::Operation::TypedLiteral {
                                base_type: x.entry_offset,
                                value: read::EndianSlice::new(&[23], LittleEndian),
                            },
                        ),
                        (&|x| x.op_convert(None), Operation::Convert(None), &|_| {
                            read::Operation::Convert {
                                base_type: read::UnitOffset(0),
                            }
                        }),
                        (
                            &|x| x.op_convert(Some(entry_id)),
                            Operation::Convert(Some(entry_id)),
                            &|x| read::Operation::Convert {
                                base_type: x.entry_offset,
                            },
                        ),
                        (
                            &|x| x.op_reinterpret(None),
                            Operation::Reinterpret(None),
                            &|_| read::Operation::Reinterpret {
                                base_type: read::UnitOffset(0),
                            },
                        ),
                        (
                            &|x| x.op_reinterpret(Some(entry_id)),
                            Operation::Reinterpret(Some(entry_id)),
                            &|x| read::Operation::Reinterpret {
                                base_type: x.entry_offset,
                            },
                        ),
                        (
                            &|x| x.op_wasm_local(1000),
                            Operation::WasmLocal(1000),
                            &|_| read::Operation::WasmLocal { index: 1000 },
                        ),
                        (
                            &|x| x.op_wasm_global(1000),
                            Operation::WasmGlobal(1000),
                            &|_| read::Operation::WasmGlobal { index: 1000 },
                        ),
                        (
                            &|x| x.op_wasm_stack(1000),
                            Operation::WasmStack(1000),
                            &|_| read::Operation::WasmStack { index: 1000 },
                        ),
                    ];

                    // Create a single expression containing all operations.
                    let mut expression = Expression::new();
                    let start_index = expression.next_index();
                    for (f, o, _) in operations {
                        f(&mut expression);
                        assert_eq!(expression.operations.last(), Some(o));
                    }

                    let bra_index = expression.op_bra();
                    let skip_index = expression.op_skip();
                    expression.op(constants::DW_OP_nop);
                    let end_index = expression.next_index();
                    expression.set_target(bra_index, start_index);
                    expression.set_target(skip_index, end_index);

                    // Create an entry containing the expression.
                    let subprogram_id = unit.add(unit.root(), constants::DW_TAG_subprogram);
                    let subprogram = unit.get_mut(subprogram_id);
                    subprogram.set(
                        constants::DW_AT_location,
                        AttributeValue::Exprloc(expression),
                    );

                    // Write the DWARF, then parse it.
                    let mut sections = Sections::new(EndianVec::new(LittleEndian));
                    dwarf.write(&mut sections).unwrap();

                    let read_dwarf = sections.read(LittleEndian);
                    let mut read_units = read_dwarf.units();
                    let read_unit_header = read_units.next().unwrap().unwrap();
                    let read_unit = read_dwarf.unit(read_unit_header).unwrap();
                    let mut read_entries = read_unit.entries();
                    let (_, read_entry) = read_entries.next_dfs().unwrap().unwrap();
                    assert_eq!(read_entry.tag(), constants::DW_TAG_compile_unit);

                    // Determine the offset of the entry that can be referenced by the expression.
                    let (_, read_entry) = read_entries.next_dfs().unwrap().unwrap();
                    assert_eq!(read_entry.tag(), constants::DW_TAG_base_type);
                    let read_state = ReadState {
                        debug_info_offset: read_entry
                            .offset()
                            .to_debug_info_offset(&read_unit.header)
                            .unwrap(),
                        entry_offset: read_entry.offset(),
                    };

                    // Get the expression.
                    let (_, read_entry) = read_entries.next_dfs().unwrap().unwrap();
                    assert_eq!(read_entry.tag(), constants::DW_TAG_subprogram);
                    let read_attr = read_entry
                        .attr_value(constants::DW_AT_location)
                        .unwrap()
                        .unwrap();
                    let read_expression = read_attr.exprloc_value().unwrap();
                    let mut read_operations = read_expression.operations(encoding);
                    for (_, _, operation) in operations {
                        assert_eq!(read_operations.next(), Ok(Some(operation(&read_state))));
                    }

                    // 4 = DW_OP_skip + i16 + DW_OP_nop
                    assert_eq!(
                        read_operations.next(),
                        Ok(Some(read::Operation::Bra {
                            target: -(read_expression.0.len() as i16) + 4
                        }))
                    );
                    // 1 = DW_OP_nop
                    assert_eq!(
                        read_operations.next(),
                        Ok(Some(read::Operation::Skip { target: 1 }))
                    );
                    assert_eq!(read_operations.next(), Ok(Some(read::Operation::Nop)));
                    assert_eq!(read_operations.next(), Ok(None));

                    let mut entry_ids = HashMap::new();
                    entry_ids.insert(read_state.debug_info_offset.into(), (unit_id, entry_id));
                    let convert_expression = Expression::from(
                        read_expression,
                        encoding,
                        Some(&read_dwarf),
                        Some(&read_unit),
                        Some(&entry_ids),
                        &|address| Some(Address::Constant(address)),
                    )
                    .unwrap();
                    let mut convert_operations = convert_expression.operations.iter();
                    for (_, operation, _) in operations {
                        assert_eq!(convert_operations.next(), Some(operation));
                    }
                    assert_eq!(
                        convert_operations.next(),
                        Some(&Operation::Branch(start_index))
                    );
                    assert_eq!(convert_operations.next(), Some(&Operation::Skip(end_index)));
                    assert_eq!(
                        convert_operations.next(),
                        Some(&Operation::Simple(constants::DW_OP_nop))
                    );
                }
            }
        }
    }

    #[test]
    #[allow(clippy::type_complexity)]
    fn test_expression_raw() {
        for version in [2, 3, 4, 5] {
            for address_size in [4, 8] {
                for format in [Format::Dwarf32, Format::Dwarf64] {
                    let encoding = Encoding {
                        format,
                        version,
                        address_size,
                    };

                    let mut dwarf = Dwarf::new();
                    let unit_id = dwarf.units.add(Unit::new(encoding, LineProgram::none()));
                    let unit = dwarf.units.get_mut(unit_id);
                    let subprogram_id = unit.add(unit.root(), constants::DW_TAG_subprogram);
                    let subprogram = unit.get_mut(subprogram_id);
                    let expression = Expression::raw(vec![constants::DW_OP_constu.0, 23]);
                    subprogram.set(
                        constants::DW_AT_location,
                        AttributeValue::Exprloc(expression),
                    );

                    // Write the DWARF, then parse it.
                    let mut sections = Sections::new(EndianVec::new(LittleEndian));
                    dwarf.write(&mut sections).unwrap();

                    let read_dwarf = sections.read(LittleEndian);
                    let mut read_units = read_dwarf.units();
                    let read_unit_header = read_units.next().unwrap().unwrap();
                    let read_unit = read_dwarf.unit(read_unit_header).unwrap();
                    let mut read_entries = read_unit.entries();
                    let (_, read_entry) = read_entries.next_dfs().unwrap().unwrap();
                    assert_eq!(read_entry.tag(), constants::DW_TAG_compile_unit);
                    let (_, read_entry) = read_entries.next_dfs().unwrap().unwrap();
                    assert_eq!(read_entry.tag(), constants::DW_TAG_subprogram);
                    let read_attr = read_entry
                        .attr_value(constants::DW_AT_location)
                        .unwrap()
                        .unwrap();
                    let read_expression = read_attr.exprloc_value().unwrap();
                    let mut read_operations = read_expression.operations(encoding);
                    assert_eq!(
                        read_operations.next(),
                        Ok(Some(read::Operation::UnsignedConstant { value: 23 }))
                    );
                    assert_eq!(read_operations.next(), Ok(None));
                }
            }
        }
    }

    #[test]
    fn test_forward_ref() {
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 5,
            address_size: 8,
        };

        let mut dwarf = Dwarf::new();
        let unit_id = dwarf.units.add(Unit::new(encoding, LineProgram::none()));
        let unit = dwarf.units.get_mut(unit_id);

        // First, create an entry that will contain the expression.
        let subprogram_id = unit.add(unit.root(), constants::DW_TAG_subprogram);

        // Now create the entry to be referenced by the expression.
        let entry_id = unit.add(unit.root(), constants::DW_TAG_const_type);

        // Create an expression containing the reference.
        let mut expression = Expression::new();
        expression.op_deref_type(2, entry_id);

        // Add the expression to the subprogram.
        unit.get_mut(subprogram_id).set(
            constants::DW_AT_location,
            AttributeValue::Exprloc(expression),
        );

        // Writing the DWARF should fail.
        let mut sections = Sections::new(EndianVec::new(LittleEndian));
        assert_eq!(
            dwarf.write(&mut sections),
            Err(Error::UnsupportedExpressionForwardReference)
        );
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

        // Create the entry to be referenced by the expression.
        let entry_id = unit.add(unit.root(), constants::DW_TAG_const_type);
        // And delete it so that it is not available when writing the expression.
        unit.get_mut(unit.root()).delete_child(entry_id);

        // Create an expression containing the reference.
        let mut expression = Expression::new();
        expression.op_deref_type(2, entry_id);

        // Create an entry containing the expression.
        let subprogram_id = unit.add(unit.root(), constants::DW_TAG_subprogram);
        unit.get_mut(subprogram_id).set(
            constants::DW_AT_location,
            AttributeValue::Exprloc(expression),
        );

        // Writing the DWARF should fail.
        let mut sections = Sections::new(EndianVec::new(LittleEndian));
        assert_eq!(
            dwarf.write(&mut sections),
            Err(Error::UnsupportedExpressionForwardReference)
        );
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

        // Create the entry to be referenced by the expression.
        let entry_id = unit.add(unit.root(), constants::DW_TAG_const_type);
        // And delete it so that it is not available when writing the expression.
        unit.get_mut(unit.root()).delete_child(entry_id);

        // Create an expression containing the reference.
        let mut expression = Expression::new();
        expression.op_call_ref(Reference::Entry(unit_id, entry_id));

        // Create an entry containing the expression.
        let subprogram_id = unit.add(unit.root(), constants::DW_TAG_subprogram);
        unit.get_mut(subprogram_id).set(
            constants::DW_AT_location,
            AttributeValue::Exprloc(expression),
        );

        // Writing the DWARF should fail.
        let mut sections = Sections::new(EndianVec::new(LittleEndian));
        assert_eq!(dwarf.write(&mut sections), Err(Error::InvalidReference));
    }
}
