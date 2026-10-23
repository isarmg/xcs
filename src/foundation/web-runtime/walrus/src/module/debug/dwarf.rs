//! transform entries in WebAssembly DWARF sections

/// `write` provides only address-based entry conversions,
/// does not provide entry-wise conversions.
/// We want to convert addresses of instructions, here will re-implement.
use gimli::*;

use super::units::DebuggingInformationCursor;

#[derive(Debug, PartialEq)]
pub enum AddressSearchPreference {
    /// Normal range comparison (inclusive start, exclusive end)
    ExclusiveFunctionEnd,
    /// Prefer treating a beginning point as the ending point of the previous function.
    InclusiveFunctionEnd,
}

/// Tombstone for addresses in code that no longer exists.
///
/// `-2` rather than `-1`: in DWARF 4 `.debug_loc` and `.debug_ranges` with
/// 4-byte addresses, a `begin` of `-1` is the base address selection marker
/// (which carries no expression length), so a `-1` tombstone desyncs every
/// reader of the rest of the section. lld makes the same choice.
pub(crate) static DEAD_CODE: u64 = 0xFFFF_FFFE;

/// DWARF convertion context
pub(crate) struct ConvertContext<'a, R: Reader<Offset = usize>> {
    /// Source DWARF debug data
    pub debug_str: &'a read::DebugStr<R>,
    pub debug_line_str: &'a read::DebugLineStr<R>,

    pub strings: &'a mut write::StringTable,
    pub line_strings: &'a mut write::LineStringTable,

    /// Address conversion function.
    /// First argument is an address in original wasm binary.
    /// If the address is mapped in transformed wasm binary, the address should be wrapped in Option::Some.
    /// If the address is not mapped, None should be returned.
    pub convert_address: &'a dyn Fn(u64, AddressSearchPreference) -> Option<write::Address>,
}

impl<'a, R> ConvertContext<'a, R>
where
    R: Reader<Offset = usize>,
{
    pub(crate) fn new(
        debug_str: &'a read::DebugStr<R>,
        debug_line_str: &'a read::DebugLineStr<R>,
        strings: &'a mut write::StringTable,
        line_strings: &'a mut write::LineStringTable,
        convert_address: &'a dyn Fn(u64, AddressSearchPreference) -> Option<write::Address>,
    ) -> Self {
        ConvertContext {
            debug_str,
            debug_line_str,
            strings,
            line_strings,
            convert_address,
        }
    }

    pub(crate) fn convert_high_pc(
        &self,
        from_unit: &mut gimli::read::EntriesCursor<R>,
        unit: &mut DebuggingInformationCursor,
    ) {
        while let (Ok(Some((_, from_debug_entry))), Some(debug_entry)) =
            (from_unit.next_dfs(), unit.next_dfs())
        {
            let low_pc = from_debug_entry
                .attr_value(constants::DW_AT_low_pc)
                .expect("low_pc");
            let high_pc = from_debug_entry
                .attr_value(constants::DW_AT_high_pc)
                .expect("high_pc");

            let (Some(AttributeValue::Addr(low_addr)), Some(AttributeValue::Udata(offset))) =
                (low_pc, high_pc)
            else {
                continue;
            };
            let new_low_pc =
                (self.convert_address)(low_addr, AddressSearchPreference::InclusiveFunctionEnd);
            let new_high_pc = (self.convert_address)(
                low_addr + offset,
                AddressSearchPreference::InclusiveFunctionEnd,
            );
            let offset = match (new_low_pc, new_high_pc) {
                (
                    Some(write::Address::Constant(new_low_pc)),
                    Some(write::Address::Constant(new_high_pc)),
                ) => new_high_pc.saturating_sub(new_low_pc),
                _ => offset,
            };
            // gimli reads any `DW_AT_high_pc` offset as `Udata`, which it
            // would then write as a variable-length LEB. Keep the fixed-width
            // form LLVM emits: consumers that patch code addresses in place
            // (binaryen's DWARF update) rely on the size not changing.
            let offset = match u32::try_from(offset) {
                Ok(offset) => write::AttributeValue::Data4(offset),
                Err(_) => write::AttributeValue::Udata(offset),
            };
            debug_entry.set(constants::DW_AT_high_pc, offset);
        }
    }

    pub(crate) fn convert_unit_line_program(
        &mut self,
        from_unit: read::Unit<R>,
    ) -> Option<write::LineProgram> {
        match from_unit.line_program {
            Some(ref from_program) => {
                let from_program = from_program.clone();
                let line_program = self
                    .convert_line_program(from_program)
                    .expect("cannot convert line program");
                Some(line_program)
            }
            None => None,
        }
    }

    /// Perform conversion in DWARF line program header.
    /// Almostly cloned from https://github.com/gimli-rs/gimli/blob/master/src/write/line.rs#L985
    fn convert_line_program_header(
        &mut self,
        from_program: &read::IncompleteLineProgram<R>,
        files: &mut Vec<write::FileId>,
    ) -> write::ConvertResult<write::LineProgram> {
        let from_header = from_program.header();
        let encoding = from_header.encoding();

        let comp_dir = match from_header.directory(0) {
            Some(comp_dir) => self.convert_line_string(comp_dir)?,
            None => write::LineString::new(&[][..], encoding, self.line_strings),
        };

        let (comp_name, comp_file_info) = match from_header.file(0) {
            Some(comp_file) => {
                if comp_file.directory_index() != 0 {
                    return Err(write::ConvertError::InvalidDirectoryIndex);
                }
                (
                    self.convert_line_string(comp_file.path_name())?,
                    Some(write::FileInfo {
                        timestamp: comp_file.timestamp(),
                        size: comp_file.size(),
                        md5: *comp_file.md5(),
                        source: None,
                    }),
                )
            }
            None => (
                write::LineString::new(&[][..], encoding, self.line_strings),
                None,
            ),
        };

        if from_header.line_base() > 0 {
            return Err(write::ConvertError::InvalidLineBase);
        }
        let mut program = write::LineProgram::new(
            encoding,
            from_header.line_encoding(),
            comp_dir,
            None,
            comp_name,
            comp_file_info,
        );

        let mut dirs = Vec::new();
        let file_skip = if from_header.version() <= 4 {
            // The first directory is implicit.
            dirs.push(program.default_directory());
            // A file index of 0 is invalid for version <= 4, but putting
            // something there makes the indexing easier.
            0
        } else {
            // We don't add the first file to `files`, but still allow
            // it to be referenced from converted instructions.
            1
        };

        for from_dir in from_header.include_directories() {
            let from_dir = self.convert_line_string(from_dir.clone())?;
            dirs.push(program.add_directory(from_dir));
        }

        for from_file in from_header.file_names().iter().skip(file_skip) {
            let from_name = self.convert_line_string(from_file.path_name())?;
            let from_dir = from_file.directory_index();
            if from_dir >= dirs.len() as u64 {
                return Err(write::ConvertError::InvalidDirectoryIndex);
            }
            let from_dir = dirs[from_dir as usize];
            let from_info = Some(write::FileInfo {
                timestamp: from_file.timestamp(),
                size: from_file.size(),
                md5: *from_file.md5(),
                source: None,
            });
            files.push(program.add_file(from_name, from_dir, from_info));
        }

        Ok(program)
    }

    /// Perform address conversion in DWARF line program entries.
    /// Almostly cloned from https://github.com/gimli-rs/gimli/blob/master/src/write/line.rs#L1066
    fn convert_line_program(
        &mut self,
        mut from_program: read::IncompleteLineProgram<R>,
    ) -> write::ConvertResult<write::LineProgram> {
        let mut files = Vec::new();
        // Create mappings in case the source has duplicate files or directories.
        let mut program = self
            .convert_line_program_header(&from_program, &mut files)
            .expect("line program header cannot be converted");

        // We can't use the `from_program.rows()` because that wouldn't let
        // us preserve address relocations.
        let mut from_row = read::LineRow::new(from_program.header());
        let mut instructions = from_program.header().instructions();
        let mut current_sequence_base_address = None;
        let mut from_base_address = 0;

        while let Some(instruction) = instructions.next_instruction(from_program.header())? {
            match instruction {
                read::LineInstruction::SetAddress(val) => {
                    if program.in_sequence() {
                        return Err(write::ConvertError::UnsupportedLineInstruction);
                    }
                    from_base_address = val;

                    from_row.execute(read::LineInstruction::SetAddress(0), &mut from_program)?;
                }
                read::LineInstruction::DefineFile(_) => {
                    return Err(write::ConvertError::UnsupportedLineInstruction);
                }
                _ => {
                    if from_row.execute(instruction, &mut from_program)? {
                        if !program.in_sequence() {
                            // begin new sequence if exists
                            current_sequence_base_address = (self.convert_address)(
                                from_base_address,
                                AddressSearchPreference::ExclusiveFunctionEnd,
                            );

                            if current_sequence_base_address.is_some() {
                                program.begin_sequence(current_sequence_base_address);
                            }
                        }

                        if let Some(write::Address::Constant(base_address)) =
                            current_sequence_base_address
                        {
                            // New offset from sequence base address in the transformed wasm binary
                            // can be different from one in the original wasm binary.
                            // Therefore, reculculating the new offset here.
                            let from_row_address = from_row.address() + from_base_address;
                            let row_address = (self.convert_address)(
                                from_row_address,
                                AddressSearchPreference::InclusiveFunctionEnd,
                            );

                            // either sequence_base_address or row_address is not resolved, ignore this entry.
                            if let Some(write::Address::Constant(address)) = row_address {
                                let address_offset = address.saturating_sub(base_address);

                                if from_row.end_sequence() {
                                    program.end_sequence(address_offset);
                                    from_base_address = from_row_address;
                                } else {
                                    program.row().address_offset = address_offset;
                                    program.row().op_index = from_row.op_index();
                                    program.row().file = {
                                        let file = from_row.file_index();
                                        if file > files.len() as u64 {
                                            return Err(write::ConvertError::InvalidFileIndex);
                                        }
                                        if file == 0 && program.version() <= 4 {
                                            return Err(write::ConvertError::InvalidFileIndex);
                                        }
                                        files[(file - 1) as usize]
                                    };
                                    program.row().line = match from_row.line() {
                                        Some(line) => line.get(),
                                        None => 0,
                                    };
                                    program.row().column = match from_row.column() {
                                        read::ColumnType::LeftEdge => 0,
                                        read::ColumnType::Column(val) => val.get(),
                                    };
                                    program.row().discriminator = from_row.discriminator();
                                    program.row().is_statement = from_row.is_stmt();
                                    program.row().basic_block = from_row.basic_block();
                                    program.row().prologue_end = from_row.prologue_end();
                                    program.row().epilogue_begin = from_row.epilogue_begin();
                                    program.row().isa = from_row.isa();
                                    program.generate_row();
                                }
                            }
                        }

                        from_row.reset(from_program.header());
                    }
                }
            };
        }
        Ok(program)
    }

    /// write::LineString::from is not public function, cloned from https://github.com/gimli-rs/gimli/blob/master/src/write/line.rs#L1131
    fn convert_line_string(
        &mut self,
        from_attr: read::AttributeValue<R>,
    ) -> write::ConvertResult<write::LineString> {
        Ok(match from_attr {
            read::AttributeValue::String(r) => write::LineString::String(r.to_slice()?.to_vec()),
            read::AttributeValue::DebugStrRef(offset) => {
                let r = self.debug_str.get_str(offset)?;
                let id = self.strings.add(r.to_slice()?);
                write::LineString::StringRef(id)
            }
            read::AttributeValue::DebugLineStrRef(offset) => {
                let r = self.debug_line_str.get_str(offset)?;
                let id = self.line_strings.add(r.to_slice()?);
                write::LineString::LineStringRef(id)
            }
            _ => return Err(write::ConvertError::UnsupportedLineStringForm),
        })
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/walrus/unit/src/module/debug/dwarf/tests_335.rs"));
