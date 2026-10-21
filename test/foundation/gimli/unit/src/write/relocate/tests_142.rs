#[cfg(test)]
mod tests {
    use super::*;
    use crate::write::EndianVec;
    use crate::{LittleEndian, SectionId};
    use alloc::vec::Vec;

    struct Section {
        writer: EndianVec<LittleEndian>,
        relocations: Vec<Relocation>,
    }

    impl RelocateWriter for Section {
        type Writer = EndianVec<LittleEndian>;

        fn writer(&self) -> &Self::Writer {
            &self.writer
        }

        fn writer_mut(&mut self) -> &mut Self::Writer {
            &mut self.writer
        }

        fn relocate(&mut self, relocation: Relocation) {
            self.relocations.push(relocation);
        }
    }

    #[test]
    fn test_relocate_writer() {
        let mut expected_data = Vec::new();
        let mut expected_relocations = Vec::new();

        let mut section = Section {
            writer: EndianVec::new(LittleEndian),
            relocations: Vec::new(),
        };

        // No relocation for plain data.
        section.write_udata(0x12345678, 4).unwrap();
        expected_data.extend_from_slice(&0x12345678u32.to_le_bytes());

        // No relocation for a constant address.
        section
            .write_address(Address::Constant(0x87654321), 4)
            .unwrap();
        expected_data.extend_from_slice(&0x87654321u32.to_le_bytes());

        // Relocation for a symbol address.
        let offset = section.len();
        section
            .write_address(
                Address::Symbol {
                    symbol: 1,
                    addend: 0x12345678,
                },
                4,
            )
            .unwrap();
        expected_data.extend_from_slice(&[0; 4]);
        expected_relocations.push(Relocation {
            offset,
            size: 4,
            target: RelocationTarget::Symbol(1),
            addend: 0x12345678,
            eh_pe: None,
        });

        // Relocation for a section offset.
        let offset = section.len();
        section
            .write_offset(0x12345678, SectionId::DebugAbbrev, 4)
            .unwrap();
        expected_data.extend_from_slice(&[0; 4]);
        expected_relocations.push(Relocation {
            offset,
            size: 4,
            target: RelocationTarget::Section(SectionId::DebugAbbrev),
            addend: 0x12345678,
            eh_pe: None,
        });

        // Relocation for a section offset at a specific offset.
        let offset = section.len();
        section.write_udata(0x12345678, 4).unwrap();
        section
            .write_offset_at(offset, 0x12345678, SectionId::DebugStr, 4)
            .unwrap();
        expected_data.extend_from_slice(&[0; 4]);
        expected_relocations.push(Relocation {
            offset,
            size: 4,
            target: RelocationTarget::Section(SectionId::DebugStr),
            addend: 0x12345678,
            eh_pe: None,
        });

        // No relocation for a constant in unwind information.
        section
            .write_eh_pointer(Address::Constant(0x87654321), constants::DW_EH_PE_absptr, 8)
            .unwrap();
        expected_data.extend_from_slice(&0x87654321u64.to_le_bytes());

        // No relocation for a relative constant in unwind information.
        let offset = section.len();
        section
            .write_eh_pointer(
                Address::Constant(offset as u64 - 8),
                constants::DW_EH_PE_pcrel | constants::DW_EH_PE_sdata4,
                8,
            )
            .unwrap();
        expected_data.extend_from_slice(&(-8i32).to_le_bytes());

        // Relocation for a symbol in unwind information.
        let offset = section.len();
        section
            .write_eh_pointer(
                Address::Symbol {
                    symbol: 2,
                    addend: 0x12345678,
                },
                constants::DW_EH_PE_pcrel | constants::DW_EH_PE_sdata4,
                8,
            )
            .unwrap();
        expected_data.extend_from_slice(&[0; 4]);
        expected_relocations.push(Relocation {
            offset,
            size: 4,
            target: RelocationTarget::Symbol(2),
            addend: 0x12345678,
            eh_pe: Some(constants::DW_EH_PE_pcrel | constants::DW_EH_PE_sdata4),
        });

        assert_eq!(section.writer.into_vec(), expected_data);
        assert_eq!(section.relocations, expected_relocations);
    }
}
