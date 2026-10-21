#[cfg(test)]
mod tests {
    use super::*;
    use crate::endianity::BigEndian;
    use test_assembler::{Endian, Section};

    #[test]
    fn test_empty() {
        let buf = EndianSlice::new(&[], BigEndian);
        let index = UnitIndex::parse(buf).unwrap();
        assert_eq!(index.version(), 0);
        assert_eq!(index.unit_count(), 0);
        assert_eq!(index.slot_count(), 0);
        assert!(index.find(0).is_none());
    }

    #[test]
    fn test_zero_slots() {
        #[rustfmt::skip]
        let section = Section::with_endian(Endian::Big)
            // Header.
            .D32(2).D32(0).D32(0).D32(0);
        let buf = section.get_contents().unwrap();
        let buf = EndianSlice::new(&buf, BigEndian);
        let index = UnitIndex::parse(buf).unwrap();
        assert_eq!(index.version(), 2);
        assert_eq!(index.unit_count(), 0);
        assert_eq!(index.slot_count(), 0);
        assert!(index.find(0).is_none());
    }

    #[test]
    fn test_version_2() {
        #[rustfmt::skip]
        let section = Section::with_endian(Endian::Big)
            // Header.
            .D32(2).D32(0).D32(0).D32(1)
            // Slots.
            .D64(0).D32(0);
        let buf = section.get_contents().unwrap();
        let buf = EndianSlice::new(&buf, BigEndian);
        let index = UnitIndex::parse(buf).unwrap();
        assert_eq!(index.version, 2);
    }

    #[test]
    fn test_version_5() {
        #[rustfmt::skip]
        let section = Section::with_endian(Endian::Big)
            // Header.
            .D16(5).D16(0).D32(0).D32(0).D32(1)
            // Slots.
            .D64(0).D32(0);
        let buf = section.get_contents().unwrap();
        let buf = EndianSlice::new(&buf, BigEndian);
        let index = UnitIndex::parse(buf).unwrap();
        assert_eq!(index.version, 5);
    }

    #[test]
    fn test_version_5_invalid() {
        #[rustfmt::skip]
        let section = Section::with_endian(Endian::Big)
            // Header.
            .D32(5).D32(0).D32(0).D32(1)
            // Slots.
            .D64(0).D32(0);
        let buf = section.get_contents().unwrap();
        let buf = EndianSlice::new(&buf, BigEndian);
        assert!(UnitIndex::parse(buf).is_err());
    }

    #[test]
    fn test_version_2_sections() {
        #[rustfmt::skip]
        let section = Section::with_endian(Endian::Big)
            // Header.
            .D32(2).D32(8).D32(1).D32(2)
            // Slots.
            .D64(0).D64(0).D32(0).D32(0)
            // Sections.
            .D32(constants::DW_SECT_V2_INFO.0)
            .D32(constants::DW_SECT_V2_TYPES.0)
            .D32(constants::DW_SECT_V2_ABBREV.0)
            .D32(constants::DW_SECT_V2_LINE.0)
            .D32(constants::DW_SECT_V2_LOC.0)
            .D32(constants::DW_SECT_V2_STR_OFFSETS.0)
            .D32(constants::DW_SECT_V2_MACINFO.0)
            .D32(constants::DW_SECT_V2_MACRO.0)
            // Offsets.
            .D32(11).D32(12).D32(13).D32(14).D32(15).D32(16).D32(17).D32(18)
            // Sizes.
            .D32(21).D32(22).D32(23).D32(24).D32(25).D32(26).D32(27).D32(28);
        let buf = section.get_contents().unwrap();
        let buf = EndianSlice::new(&buf, BigEndian);
        let index = UnitIndex::parse(buf).unwrap();
        assert_eq!(index.section_count, 8);
        assert_eq!(
            index.sections,
            [
                IndexSectionId::DebugInfo,
                IndexSectionId::DebugTypes,
                IndexSectionId::DebugAbbrev,
                IndexSectionId::DebugLine,
                IndexSectionId::DebugLoc,
                IndexSectionId::DebugStrOffsets,
                IndexSectionId::DebugMacinfo,
                IndexSectionId::DebugMacro,
            ]
        );
        #[rustfmt::skip]
        let expect = [
            UnitIndexSection { section: IndexSectionId::DebugInfo, offset: 11, size: 21 },
            UnitIndexSection { section: IndexSectionId::DebugTypes, offset: 12, size: 22 },
            UnitIndexSection { section: IndexSectionId::DebugAbbrev, offset: 13, size: 23 },
            UnitIndexSection { section: IndexSectionId::DebugLine, offset: 14, size: 24 },
            UnitIndexSection { section: IndexSectionId::DebugLoc, offset: 15, size: 25 },
            UnitIndexSection { section: IndexSectionId::DebugStrOffsets, offset: 16, size: 26 },
            UnitIndexSection { section: IndexSectionId::DebugMacinfo, offset: 17, size: 27 },
            UnitIndexSection { section: IndexSectionId::DebugMacro, offset: 18, size: 28 },
        ];
        let mut sections = index.sections(1).unwrap();
        for section in &expect {
            assert_eq!(*section, sections.next().unwrap());
        }
        assert!(sections.next().is_none());
    }

    #[test]
    fn test_version_5_sections() {
        #[rustfmt::skip]
        let section = Section::with_endian(Endian::Big)
            // Header.
            .D16(5).D16(0).D32(7).D32(1).D32(2)
            // Slots.
            .D64(0).D64(0).D32(0).D32(0)
            // Sections.
            .D32(constants::DW_SECT_INFO.0)
            .D32(constants::DW_SECT_ABBREV.0)
            .D32(constants::DW_SECT_LINE.0)
            .D32(constants::DW_SECT_LOCLISTS.0)
            .D32(constants::DW_SECT_STR_OFFSETS.0)
            .D32(constants::DW_SECT_MACRO.0)
            .D32(constants::DW_SECT_RNGLISTS.0)
            // Offsets.
            .D32(11).D32(12).D32(13).D32(14).D32(15).D32(16).D32(17)
            // Sizes.
            .D32(21).D32(22).D32(23).D32(24).D32(25).D32(26).D32(27);
        let buf = section.get_contents().unwrap();
        let buf = EndianSlice::new(&buf, BigEndian);
        let index = UnitIndex::parse(buf).unwrap();
        assert_eq!(index.section_count, 7);
        assert_eq!(
            index.sections[..7],
            [
                IndexSectionId::DebugInfo,
                IndexSectionId::DebugAbbrev,
                IndexSectionId::DebugLine,
                IndexSectionId::DebugLocLists,
                IndexSectionId::DebugStrOffsets,
                IndexSectionId::DebugMacro,
                IndexSectionId::DebugRngLists,
            ]
        );
        #[rustfmt::skip]
        let expect = [
            UnitIndexSection { section: IndexSectionId::DebugInfo, offset: 11, size: 21 },
            UnitIndexSection { section: IndexSectionId::DebugAbbrev, offset: 12, size: 22 },
            UnitIndexSection { section: IndexSectionId::DebugLine, offset: 13, size: 23 },
            UnitIndexSection { section: IndexSectionId::DebugLocLists, offset: 14, size: 24 },
            UnitIndexSection { section: IndexSectionId::DebugStrOffsets, offset: 15, size: 25 },
            UnitIndexSection { section: IndexSectionId::DebugMacro, offset: 16, size: 26 },
            UnitIndexSection { section: IndexSectionId::DebugRngLists, offset: 17, size: 27 },
        ];
        let mut sections = index.sections(1).unwrap();
        for section in &expect {
            assert_eq!(*section, sections.next().unwrap());
        }
        assert!(sections.next().is_none());

        assert!(index.sections(0).is_err());
        assert!(index.sections(2).is_err());
    }

    #[test]
    fn test_hash() {
        #[rustfmt::skip]
        let section = Section::with_endian(Endian::Big)
            // Header.
            .D16(5).D16(0).D32(2).D32(3).D32(4)
            // Slots.
            .D64(0xffff_fff2_ffff_fff1)
            .D64(0xffff_fff0_ffff_fff1)
            .D64(0xffff_fff1_ffff_fff1)
            .D64(0)
            .D32(3).D32(1).D32(2).D32(0)
            // Sections.
            .D32(constants::DW_SECT_INFO.0)
            .D32(constants::DW_SECT_ABBREV.0)
            // Offsets.
            .D32(0).D32(0).D32(0).D32(0).D32(0).D32(0)
            // Sizes.
            .D32(0).D32(0).D32(0).D32(0).D32(0).D32(0);
        let buf = section.get_contents().unwrap();
        let buf = EndianSlice::new(&buf, BigEndian);
        let index = UnitIndex::parse(buf).unwrap();
        assert_eq!(index.version(), 5);
        assert_eq!(index.slot_count(), 4);
        assert_eq!(index.unit_count(), 3);
        assert_eq!(index.section_count(), 2);
        assert_eq!(index.find(0xffff_fff0_ffff_fff1), Some(1));
        assert_eq!(index.find(0xffff_fff1_ffff_fff1), Some(2));
        assert_eq!(index.find(0xffff_fff2_ffff_fff1), Some(3));
        assert_eq!(index.find(0xffff_fff3_ffff_fff1), None);
    }

    #[test]
    fn test_cu_index() {
        #[rustfmt::skip]
        let section = Section::with_endian(Endian::Big)
            // Header.
            .D16(5).D16(0).D32(0).D32(0).D32(1)
            // Slots.
            .D64(0).D32(0);
        let buf = section.get_contents().unwrap();
        let cu_index = DebugCuIndex::new(&buf, BigEndian);
        let index = cu_index.index().unwrap();
        assert_eq!(index.version, 5);
    }

    #[test]
    fn test_tu_index() {
        #[rustfmt::skip]
        let section = Section::with_endian(Endian::Big)
            // Header.
            .D16(5).D16(0).D32(0).D32(0).D32(1)
            // Slots.
            .D64(0).D32(0);
        let buf = section.get_contents().unwrap();
        let tu_index = DebugTuIndex::new(&buf, BigEndian);
        let index = tu_index.index().unwrap();
        assert_eq!(index.version, 5);
    }
}
