#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::GimliSectionMethods;
    use crate::LittleEndian;
    use test_assembler::{Endian, Label, LabelMaker, Section};

    #[test]
    fn test_get_str_offset() {
        for format in [Format::Dwarf32, Format::Dwarf64] {
            let zero = Label::new();
            let length = Label::new();
            let start = Label::new();
            let first = Label::new();
            let end = Label::new();
            let mut section = Section::with_endian(Endian::Little)
                .mark(&zero)
                .initial_length(format, &length, &start)
                .D16(5)
                .D16(0)
                .mark(&first);
            for i in 0..20 {
                section = section.word(format.word_size(), 1000 + i);
            }
            section = section.mark(&end);
            length.set_const((&end - &start) as u64);

            let section = section.get_contents().unwrap();
            let debug_str_offsets = DebugStrOffsets::from(EndianSlice::new(&section, LittleEndian));
            let base = DebugStrOffsetsBase((&first - &zero) as usize);

            assert_eq!(
                debug_str_offsets.get_str_offset(format, base, DebugStrOffsetsIndex(0)),
                Ok(DebugStrOffset(1000))
            );
            assert_eq!(
                debug_str_offsets.get_str_offset(format, base, DebugStrOffsetsIndex(19)),
                Ok(DebugStrOffset(1019))
            );
        }
    }
}
