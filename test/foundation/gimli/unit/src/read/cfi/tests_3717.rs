#[cfg(test)]
mod tests {
    use super::*;
    use super::{parse_cfi_entry, AugmentationData, RegisterRuleMap, UnwindContext};
    use crate::common::Format;
    use crate::constants;
    use crate::endianity::{BigEndian, Endianity, LittleEndian, NativeEndian};
    use crate::read::{
        EndianSlice, Error, Pointer, ReaderOffsetId, Result, Section as ReadSection,
    };
    use crate::test_util::GimliSectionMethods;
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    use core::marker::PhantomData;
    use core::mem;
    use test_assembler::{Endian, Label, LabelMaker, LabelOrNum, Section, ToLabelOrNum};

    // Ensure each test tries to read the same section kind that it wrote.
    #[derive(Clone, Copy)]
    struct SectionKind<Section>(PhantomData<Section>);

    impl<T> SectionKind<T> {
        fn endian<'input, E>(self) -> Endian
        where
            E: Endianity,
            T: UnwindSection<EndianSlice<'input, E>>,
            T::Offset: UnwindOffset<usize>,
        {
            if E::default().is_big_endian() {
                Endian::Big
            } else {
                Endian::Little
            }
        }

        fn section<'input, E>(self, contents: &'input [u8]) -> T
        where
            E: Endianity,
            T: UnwindSection<EndianSlice<'input, E>> + ReadSection<EndianSlice<'input, E>>,
            T::Offset: UnwindOffset<usize>,
        {
            EndianSlice::new(contents, E::default()).into()
        }
    }

    fn debug_frame_le<'a>() -> SectionKind<DebugFrame<EndianSlice<'a, LittleEndian>>> {
        SectionKind(PhantomData)
    }

    fn debug_frame_be<'a>() -> SectionKind<DebugFrame<EndianSlice<'a, BigEndian>>> {
        SectionKind(PhantomData)
    }

    fn eh_frame_le<'a>() -> SectionKind<EhFrame<EndianSlice<'a, LittleEndian>>> {
        SectionKind(PhantomData)
    }

    fn parse_fde<Section, O, F, R>(
        section: Section,
        input: &mut R,
        get_cie: F,
    ) -> Result<FrameDescriptionEntry<R>>
    where
        R: Reader,
        Section: UnwindSection<R, Offset = O>,
        O: UnwindOffset<R::Offset>,
        F: FnMut(&Section, &BaseAddresses, O) -> Result<CommonInformationEntry<R>>,
    {
        let bases = Default::default();
        match parse_cfi_entry(&bases, &section, input) {
            Ok(Some(CieOrFde::Fde(partial))) => partial.parse(get_cie),
            Ok(_) => Err(Error::NoEntryAtGivenOffset),
            Err(e) => Err(e),
        }
    }

    // Mixin methods for `Section` to help define binary test data.

    trait CfiSectionMethods: GimliSectionMethods {
        fn cie<'aug, 'input, E, T>(
            self,
            _kind: SectionKind<T>,
            augmentation: Option<&'aug str>,
            cie: &mut CommonInformationEntry<EndianSlice<'input, E>>,
        ) -> Self
        where
            E: Endianity,
            T: UnwindSection<EndianSlice<'input, E>>,
            T::Offset: UnwindOffset;
        fn fde<'a, 'input, E, T, L>(
            self,
            _kind: SectionKind<T>,
            cie_offset: L,
            fde: &mut FrameDescriptionEntry<EndianSlice<'input, E>>,
        ) -> Self
        where
            E: Endianity,
            T: UnwindSection<EndianSlice<'input, E>>,
            T::Offset: UnwindOffset,
            L: ToLabelOrNum<'a, u64>;
    }

    impl CfiSectionMethods for Section {
        fn cie<'aug, 'input, E, T>(
            self,
            _kind: SectionKind<T>,
            augmentation: Option<&'aug str>,
            cie: &mut CommonInformationEntry<EndianSlice<'input, E>>,
        ) -> Self
        where
            E: Endianity,
            T: UnwindSection<EndianSlice<'input, E>>,
            T::Offset: UnwindOffset,
        {
            cie.offset = self.size() as _;
            let length = Label::new();
            let start = Label::new();
            let end = Label::new();

            let section = match cie.format {
                Format::Dwarf32 => self.D32(&length).mark(&start).D32(0xffff_ffff),
                Format::Dwarf64 => {
                    let section = self.D32(0xffff_ffff);
                    section.D64(&length).mark(&start).D64(0xffff_ffff_ffff_ffff)
                }
            };

            let mut section = section.D8(cie.version);

            if let Some(augmentation) = augmentation {
                section = section.append_bytes(augmentation.as_bytes());
            }

            // Null terminator for augmentation string.
            let section = section.D8(0);

            let section = if T::has_address_and_segment_sizes(cie.version) {
                section.D8(cie.address_size).D8(0)
            } else {
                section
            };

            let section = section
                .uleb(cie.code_alignment_factor)
                .sleb(cie.data_alignment_factor)
                .uleb(cie.return_address_register.0.into())
                .append_bytes(cie.initial_instructions.slice())
                .mark(&end);

            cie.length = (&end - &start) as usize;
            length.set_const(cie.length as u64);

            section
        }

        fn fde<'a, 'input, E, T, L>(
            self,
            _kind: SectionKind<T>,
            cie_offset: L,
            fde: &mut FrameDescriptionEntry<EndianSlice<'input, E>>,
        ) -> Self
        where
            E: Endianity,
            T: UnwindSection<EndianSlice<'input, E>>,
            T::Offset: UnwindOffset,
            L: ToLabelOrNum<'a, u64>,
        {
            fde.offset = self.size() as _;
            let length = Label::new();
            let start = Label::new();
            let end = Label::new();

            assert_eq!(fde.format, fde.cie.format);

            let section = match T::cie_offset_encoding(fde.format) {
                CieOffsetEncoding::U32 => {
                    let section = self.D32(&length).mark(&start);
                    match cie_offset.to_labelornum() {
                        LabelOrNum::Label(ref l) => section.D32(l),
                        LabelOrNum::Num(o) => section.D32(o as u32),
                    }
                }
                CieOffsetEncoding::U64 => {
                    let section = self.D32(0xffff_ffff);
                    section.D64(&length).mark(&start).D64(cie_offset)
                }
            };

            let section = match fde.cie.address_size {
                4 => section
                    .D32(fde.initial_address() as u32)
                    .D32(fde.len() as u32),
                8 => section.D64(fde.initial_address()).D64(fde.len()),
                x => panic!("Unsupported address size: {}", x),
            };

            let section = if let Some(ref augmentation) = fde.augmentation {
                let cie_aug = fde
                    .cie
                    .augmentation
                    .expect("FDE has augmentation, but CIE doesn't");

                if let Some(lsda) = augmentation.lsda {
                    // We only support writing `DW_EH_PE_absptr` here.
                    assert_eq!(
                        cie_aug
                            .lsda
                            .expect("FDE has lsda, but CIE doesn't")
                            .format(),
                        constants::DW_EH_PE_absptr
                    );

                    // Augmentation data length
                    let section = section.uleb(u64::from(fde.cie.address_size));
                    match fde.cie.address_size {
                        4 => section.D32({
                            let x: u64 = lsda.pointer();
                            x as u32
                        }),
                        8 => section.D64({
                            let x: u64 = lsda.pointer();
                            x
                        }),
                        x => panic!("Unsupported address size: {}", x),
                    }
                } else {
                    // Even if we don't have any augmentation data, if there is
                    // an augmentation defined, we need to put the length in.
                    section.uleb(0)
                }
            } else {
                section
            };

            let section = section.append_bytes(fde.instructions.slice()).mark(&end);

            fde.length = (&end - &start) as usize;
            length.set_const(fde.length as u64);

            section
        }
    }

    trait ResultExt {
        fn map_eof(self, input: &[u8]) -> Self;
    }

    impl<T> ResultExt for Result<T> {
        fn map_eof(self, input: &[u8]) -> Self {
            match self {
                Err(Error::UnexpectedEof(id)) => {
                    let id = ReaderOffsetId(id.0 - input.as_ptr() as u64);
                    Err(Error::UnexpectedEof(id))
                }
                r => r,
            }
        }
    }

    fn assert_parse_cie<'input, E>(
        kind: SectionKind<DebugFrame<EndianSlice<'input, E>>>,
        section: Section,
        address_size: u8,
        expected: Result<(
            EndianSlice<'input, E>,
            CommonInformationEntry<EndianSlice<'input, E>>,
        )>,
    ) where
        E: Endianity,
    {
        let section = section.get_contents().unwrap();
        let mut debug_frame = kind.section(&section);
        debug_frame.set_address_size(address_size);
        let input = &mut EndianSlice::new(&section, E::default());
        let bases = Default::default();
        let result = CommonInformationEntry::parse(&bases, &debug_frame, input);
        let result = result.map(|cie| (*input, cie)).map_eof(&section);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_cie_incomplete_length_32() {
        let kind = debug_frame_le();
        let section = Section::with_endian(kind.endian()).L16(5);
        assert_parse_cie(
            kind,
            section,
            8,
            Err(Error::UnexpectedEof(ReaderOffsetId(0))),
        );
    }

    #[test]
    fn test_parse_cie_incomplete_length_64() {
        let kind = debug_frame_le();
        let section = Section::with_endian(kind.endian())
            .L32(0xffff_ffff)
            .L32(12345);
        assert_parse_cie(
            kind,
            section,
            8,
            Err(Error::UnexpectedEof(ReaderOffsetId(4))),
        );
    }

    #[test]
    fn test_parse_cie_incomplete_id_32() {
        let kind = debug_frame_be();
        let section = Section::with_endian(kind.endian())
            // The length is not large enough to contain the ID.
            .B32(3)
            .B32(0xffff_ffff);
        assert_parse_cie(
            kind,
            section,
            8,
            Err(Error::UnexpectedEof(ReaderOffsetId(4))),
        );
    }

    #[test]
    fn test_parse_cie_bad_id_32() {
        let kind = debug_frame_be();
        let section = Section::with_endian(kind.endian())
            // Initial length
            .B32(4)
            // Not the CIE Id.
            .B32(0xbad1_bad2);
        assert_parse_cie(kind, section, 8, Err(Error::NotCieId));
    }

    #[test]
    fn test_parse_cie_32_bad_version() {
        let mut cie = CommonInformationEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            version: 99,
            augmentation: None,
            address_size: 4,
            code_alignment_factor: 1,
            data_alignment_factor: 2,
            return_address_register: Register(3),
            initial_instructions: EndianSlice::new(&[], LittleEndian),
        };

        let kind = debug_frame_le();
        let section = Section::with_endian(kind.endian()).cie(kind, None, &mut cie);
        assert_parse_cie(kind, section, 4, Err(Error::UnknownVersion(99)));
    }

    #[test]
    fn test_parse_cie_unknown_augmentation() {
        let length = Label::new();
        let start = Label::new();
        let end = Label::new();

        let augmentation = "replicant";
        let expected_rest = [1, 2, 3];

        let kind = debug_frame_le();
        let section = Section::with_endian(kind.endian())
            // Initial length
            .L32(&length)
            .mark(&start)
            // CIE Id
            .L32(0xffff_ffff)
            // Version
            .D8(4)
            // Augmentation
            .append_bytes(augmentation.as_bytes())
            // Null terminator
            .D8(0)
            // Extra augmented data that we can't understand.
            .L32(1)
            .L32(2)
            .L32(3)
            .L32(4)
            .L32(5)
            .L32(6)
            .mark(&end)
            .append_bytes(&expected_rest);

        let expected_length = (&end - &start) as u64;
        length.set_const(expected_length);

        assert_parse_cie(kind, section, 8, Err(Error::UnknownAugmentation));
    }

    fn test_parse_cie(format: Format, version: u8, address_size: u8) {
        let expected_rest = [1, 2, 3, 4, 5, 6, 7, 8, 9];
        let expected_instrs: Vec<_> = (0..4).map(|_| constants::DW_CFA_nop.0).collect();

        let mut cie = CommonInformationEntry {
            offset: 0,
            length: 0,
            format,
            version,
            augmentation: None,
            address_size,
            code_alignment_factor: 16,
            data_alignment_factor: 32,
            return_address_register: Register(1),
            initial_instructions: EndianSlice::new(&expected_instrs, LittleEndian),
        };

        let kind = debug_frame_le();
        let section = Section::with_endian(kind.endian())
            .cie(kind, None, &mut cie)
            .append_bytes(&expected_rest);

        assert_parse_cie(
            kind,
            section,
            address_size,
            Ok((EndianSlice::new(&expected_rest, LittleEndian), cie)),
        );
    }

    #[test]
    fn test_parse_cie_32_ok() {
        test_parse_cie(Format::Dwarf32, 1, 4);
        test_parse_cie(Format::Dwarf32, 1, 8);
        test_parse_cie(Format::Dwarf32, 4, 4);
        test_parse_cie(Format::Dwarf32, 4, 8);
    }

    #[test]
    fn test_parse_cie_64_ok() {
        test_parse_cie(Format::Dwarf64, 1, 4);
        test_parse_cie(Format::Dwarf64, 1, 8);
        test_parse_cie(Format::Dwarf64, 4, 4);
        test_parse_cie(Format::Dwarf64, 4, 8);
    }

    #[test]
    fn test_parse_cie_length_too_big() {
        let expected_instrs: Vec<_> = (0..13).map(|_| constants::DW_CFA_nop.0).collect();

        let mut cie = CommonInformationEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            version: 4,
            augmentation: None,
            address_size: 4,
            code_alignment_factor: 0,
            data_alignment_factor: 0,
            return_address_register: Register(3),
            initial_instructions: EndianSlice::new(&expected_instrs, LittleEndian),
        };

        let kind = debug_frame_le();
        let section = Section::with_endian(kind.endian()).cie(kind, None, &mut cie);

        let mut contents = section.get_contents().unwrap();

        // Overwrite the length to be too big.
        contents[0] = 0;
        contents[1] = 0;
        contents[2] = 0;
        contents[3] = 255;

        let debug_frame = DebugFrame::new(&contents, LittleEndian);
        let bases = Default::default();
        assert_eq!(
            CommonInformationEntry::parse(
                &bases,
                &debug_frame,
                &mut EndianSlice::new(&contents, LittleEndian)
            )
            .map_eof(&contents),
            Err(Error::UnexpectedEof(ReaderOffsetId(4)))
        );
    }

    #[test]
    fn test_parse_fde_incomplete_length_32() {
        let kind = debug_frame_le();
        let section = Section::with_endian(kind.endian()).L16(5);
        let section = section.get_contents().unwrap();
        let debug_frame = kind.section(&section);
        let rest = &mut EndianSlice::new(&section, LittleEndian);
        assert_eq!(
            parse_fde(debug_frame, rest, UnwindSection::cie_from_offset).map_eof(&section),
            Err(Error::UnexpectedEof(ReaderOffsetId(0)))
        );
    }

    #[test]
    fn test_parse_fde_incomplete_length_64() {
        let kind = debug_frame_le();
        let section = Section::with_endian(kind.endian())
            .L32(0xffff_ffff)
            .L32(12345);
        let section = section.get_contents().unwrap();
        let debug_frame = kind.section(&section);
        let rest = &mut EndianSlice::new(&section, LittleEndian);
        assert_eq!(
            parse_fde(debug_frame, rest, UnwindSection::cie_from_offset).map_eof(&section),
            Err(Error::UnexpectedEof(ReaderOffsetId(4)))
        );
    }

    #[test]
    fn test_parse_fde_incomplete_cie_pointer_32() {
        let kind = debug_frame_be();
        let section = Section::with_endian(kind.endian())
            // The length is not large enough to contain the CIE pointer.
            .B32(3)
            .B32(1994);
        let section = section.get_contents().unwrap();
        let debug_frame = kind.section(&section);
        let rest = &mut EndianSlice::new(&section, BigEndian);
        assert_eq!(
            parse_fde(debug_frame, rest, UnwindSection::cie_from_offset).map_eof(&section),
            Err(Error::UnexpectedEof(ReaderOffsetId(4)))
        );
    }

    #[test]
    fn test_parse_fde_32_ok() {
        let expected_rest = [1, 2, 3, 4, 5, 6, 7, 8, 9];
        let cie_offset = 0xbad0_bad1;
        let expected_instrs: Vec<_> = (0..7).map(|_| constants::DW_CFA_nop.0).collect();

        let cie = CommonInformationEntry {
            offset: 0,
            length: 100,
            format: Format::Dwarf32,
            version: 4,
            augmentation: None,
            // DWARF32 with a 64 bit address size! Holy moly!
            address_size: 8,
            code_alignment_factor: 3,
            data_alignment_factor: 2,
            return_address_register: Register(1),
            initial_instructions: EndianSlice::new(&[], LittleEndian),
        };

        let mut fde = FrameDescriptionEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            cie: cie.clone(),
            initial_address: 0xfeed_beef,
            address_range: 39,
            augmentation: None,
            instructions: EndianSlice::new(&expected_instrs, LittleEndian),
        };

        let kind = debug_frame_le();
        let section = Section::with_endian(kind.endian())
            .fde(kind, cie_offset, &mut fde)
            .append_bytes(&expected_rest);

        let section = section.get_contents().unwrap();
        let debug_frame = kind.section(&section);
        let rest = &mut EndianSlice::new(&section, LittleEndian);

        let get_cie = |_: &_, _: &_, offset| {
            assert_eq!(offset, DebugFrameOffset(cie_offset as usize));
            Ok(cie.clone())
        };

        assert_eq!(parse_fde(debug_frame, rest, get_cie), Ok(fde));
        assert_eq!(*rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_fde_64_ok() {
        let expected_rest = [1, 2, 3, 4, 5, 6, 7, 8, 9];
        let cie_offset = 0xbad0_bad1;
        let expected_instrs: Vec<_> = (0..7).map(|_| constants::DW_CFA_nop.0).collect();

        let cie = CommonInformationEntry {
            offset: 0,
            length: 100,
            format: Format::Dwarf64,
            version: 4,
            augmentation: None,
            address_size: 8,
            code_alignment_factor: 3,
            data_alignment_factor: 2,
            return_address_register: Register(1),
            initial_instructions: EndianSlice::new(&[], LittleEndian),
        };

        let mut fde = FrameDescriptionEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf64,
            cie: cie.clone(),
            initial_address: 0xfeed_beef,
            address_range: 999,
            augmentation: None,
            instructions: EndianSlice::new(&expected_instrs, LittleEndian),
        };

        let kind = debug_frame_le();
        let section = Section::with_endian(kind.endian())
            .fde(kind, cie_offset, &mut fde)
            .append_bytes(&expected_rest);

        let section = section.get_contents().unwrap();
        let debug_frame = kind.section(&section);
        let rest = &mut EndianSlice::new(&section, LittleEndian);

        let get_cie = |_: &_, _: &_, offset| {
            assert_eq!(offset, DebugFrameOffset(cie_offset as usize));
            Ok(cie.clone())
        };

        assert_eq!(parse_fde(debug_frame, rest, get_cie), Ok(fde));
        assert_eq!(*rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_entry_on_cie_32_ok() {
        let expected_rest = [1, 2, 3, 4, 5, 6, 7, 8, 9];
        let expected_instrs: Vec<_> = (0..4).map(|_| constants::DW_CFA_nop.0).collect();

        let mut cie = CommonInformationEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            version: 4,
            augmentation: None,
            address_size: 4,
            code_alignment_factor: 16,
            data_alignment_factor: 32,
            return_address_register: Register(1),
            initial_instructions: EndianSlice::new(&expected_instrs, BigEndian),
        };

        let kind = debug_frame_be();
        let section = Section::with_endian(kind.endian())
            .cie(kind, None, &mut cie)
            .append_bytes(&expected_rest);
        let section = section.get_contents().unwrap();
        let debug_frame = kind.section(&section);
        let rest = &mut EndianSlice::new(&section, BigEndian);

        let bases = Default::default();
        assert_eq!(
            parse_cfi_entry(&bases, &debug_frame, rest),
            Ok(Some(CieOrFde::Cie(cie)))
        );
        assert_eq!(*rest, EndianSlice::new(&expected_rest, BigEndian));
    }

    #[test]
    fn test_parse_cfi_entry_on_fde_32_ok() {
        let cie_offset = 0x1234_5678;
        let expected_rest = [1, 2, 3, 4, 5, 6, 7, 8, 9];
        let expected_instrs: Vec<_> = (0..4).map(|_| constants::DW_CFA_nop.0).collect();

        let cie = CommonInformationEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            version: 4,
            augmentation: None,
            address_size: 4,
            code_alignment_factor: 16,
            data_alignment_factor: 32,
            return_address_register: Register(1),
            initial_instructions: EndianSlice::new(&[], BigEndian),
        };

        let mut fde = FrameDescriptionEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            cie: cie.clone(),
            initial_address: 0xfeed_beef,
            address_range: 39,
            augmentation: None,
            instructions: EndianSlice::new(&expected_instrs, BigEndian),
        };

        let kind = debug_frame_be();
        let section = Section::with_endian(kind.endian())
            .fde(kind, cie_offset, &mut fde)
            .append_bytes(&expected_rest);

        let section = section.get_contents().unwrap();
        let debug_frame = kind.section(&section);
        let rest = &mut EndianSlice::new(&section, BigEndian);

        let bases = Default::default();
        match parse_cfi_entry(&bases, &debug_frame, rest) {
            Ok(Some(CieOrFde::Fde(partial))) => {
                assert_eq!(*rest, EndianSlice::new(&expected_rest, BigEndian));

                assert_eq!(partial.length, fde.length);
                assert_eq!(partial.format, fde.format);
                assert_eq!(partial.cie_offset, DebugFrameOffset(cie_offset as usize));

                let get_cie = |_: &_, _: &_, offset| {
                    assert_eq!(offset, DebugFrameOffset(cie_offset as usize));
                    Ok(cie.clone())
                };

                assert_eq!(partial.parse(get_cie), Ok(fde));
            }
            otherwise => panic!("Unexpected result: {:#?}", otherwise),
        }
    }

    #[test]
    fn test_cfi_entries_iter() {
        let expected_instrs1: Vec<_> = (0..4).map(|_| constants::DW_CFA_nop.0).collect();

        let expected_instrs2: Vec<_> = (0..8).map(|_| constants::DW_CFA_nop.0).collect();

        let expected_instrs3: Vec<_> = (0..12).map(|_| constants::DW_CFA_nop.0).collect();

        let expected_instrs4: Vec<_> = (0..16).map(|_| constants::DW_CFA_nop.0).collect();

        let mut cie1 = CommonInformationEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            version: 4,
            augmentation: None,
            address_size: 4,
            code_alignment_factor: 1,
            data_alignment_factor: 2,
            return_address_register: Register(3),
            initial_instructions: EndianSlice::new(&expected_instrs1, BigEndian),
        };

        let mut cie2 = CommonInformationEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            version: 4,
            augmentation: None,
            address_size: 4,
            code_alignment_factor: 3,
            data_alignment_factor: 2,
            return_address_register: Register(1),
            initial_instructions: EndianSlice::new(&expected_instrs2, BigEndian),
        };

        let cie1_location = Label::new();
        let cie2_location = Label::new();

        // Write the CIEs first so that their length gets set before we clone
        // them into the FDEs and our equality assertions down the line end up
        // with all the CIEs always having he correct length.
        let kind = debug_frame_be();
        let section = Section::with_endian(kind.endian())
            .mark(&cie1_location)
            .cie(kind, None, &mut cie1)
            .mark(&cie2_location)
            .cie(kind, None, &mut cie2);

        let mut fde1 = FrameDescriptionEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            cie: cie1.clone(),
            initial_address: 0xfeed_beef,
            address_range: 39,
            augmentation: None,
            instructions: EndianSlice::new(&expected_instrs3, BigEndian),
        };

        let mut fde2 = FrameDescriptionEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            cie: cie2.clone(),
            initial_address: 0xfeed_face,
            address_range: 9000,
            augmentation: None,
            instructions: EndianSlice::new(&expected_instrs4, BigEndian),
        };

        let section =
            section
                .fde(kind, &cie1_location, &mut fde1)
                .fde(kind, &cie2_location, &mut fde2);

        section.start().set_const(0);

        let cie1_offset = cie1_location.value().unwrap() as usize;
        let cie2_offset = cie2_location.value().unwrap() as usize;

        let contents = section.get_contents().unwrap();
        let debug_frame = kind.section(&contents);

        let bases = Default::default();
        let mut entries = debug_frame.entries(&bases);

        assert_eq!(entries.next(), Ok(Some(CieOrFde::Cie(cie1.clone()))));
        assert_eq!(entries.next(), Ok(Some(CieOrFde::Cie(cie2.clone()))));

        match entries.next() {
            Ok(Some(CieOrFde::Fde(partial))) => {
                assert_eq!(partial.length, fde1.length);
                assert_eq!(partial.format, fde1.format);
                assert_eq!(partial.cie_offset, DebugFrameOffset(cie1_offset));

                let get_cie = |_: &_, _: &_, offset| {
                    assert_eq!(offset, DebugFrameOffset(cie1_offset));
                    Ok(cie1.clone())
                };
                assert_eq!(partial.parse(get_cie), Ok(fde1));
            }
            otherwise => panic!("Unexpected result: {:#?}", otherwise),
        }

        match entries.next() {
            Ok(Some(CieOrFde::Fde(partial))) => {
                assert_eq!(partial.length, fde2.length);
                assert_eq!(partial.format, fde2.format);
                assert_eq!(partial.cie_offset, DebugFrameOffset(cie2_offset));

                let get_cie = |_: &_, _: &_, offset| {
                    assert_eq!(offset, DebugFrameOffset(cie2_offset));
                    Ok(cie2.clone())
                };
                assert_eq!(partial.parse(get_cie), Ok(fde2));
            }
            otherwise => panic!("Unexpected result: {:#?}", otherwise),
        }

        assert_eq!(entries.next(), Ok(None));
    }

    #[test]
    fn test_parse_cie_from_offset() {
        let filler = [1, 2, 3, 4, 5, 6, 7, 8, 9];
        let instrs: Vec<_> = (0..5).map(|_| constants::DW_CFA_nop.0).collect();

        let mut cie = CommonInformationEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf64,
            version: 4,
            augmentation: None,
            address_size: 4,
            code_alignment_factor: 4,
            data_alignment_factor: 8,
            return_address_register: Register(12),
            initial_instructions: EndianSlice::new(&instrs, LittleEndian),
        };

        let cie_location = Label::new();

        let kind = debug_frame_le();
        let section = Section::with_endian(kind.endian())
            .append_bytes(&filler)
            .mark(&cie_location)
            .cie(kind, None, &mut cie)
            .append_bytes(&filler);

        section.start().set_const(0);

        let cie_offset = DebugFrameOffset(cie_location.value().unwrap() as usize);

        let contents = section.get_contents().unwrap();
        let debug_frame = kind.section(&contents);
        let bases = Default::default();

        assert_eq!(debug_frame.cie_from_offset(&bases, cie_offset), Ok(cie));
    }

    fn parse_cfi_instruction<R: Reader + Default>(
        input: &mut R,
        address_size: u8,
    ) -> Result<CallFrameInstruction<R::Offset>> {
        let section = input.clone();
        let parameters = &PointerEncodingParameters {
            bases: &SectionBaseAddresses::default(),
            func_base: None,
            address_size,
            section: &section,
        };
        CallFrameInstruction::parse(input, None, parameters, Vendor::Default)
    }

    #[test]
    fn test_parse_cfi_instruction_advance_loc() {
        let expected_rest = [1, 2, 3, 4];
        let expected_delta = 42;
        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_advance_loc.0 | expected_delta)
            .append_bytes(&expected_rest);
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);
        assert_eq!(
            parse_cfi_instruction(input, 8),
            Ok(CallFrameInstruction::AdvanceLoc {
                delta: u32::from(expected_delta),
            })
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_offset() {
        let expected_rest = [1, 2, 3, 4];
        let expected_reg = 3;
        let expected_offset = 1997;
        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_offset.0 | expected_reg)
            .uleb(expected_offset)
            .append_bytes(&expected_rest);
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);
        assert_eq!(
            parse_cfi_instruction(input, 8),
            Ok(CallFrameInstruction::Offset {
                register: Register(expected_reg.into()),
                factored_offset: expected_offset,
            })
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_restore() {
        let expected_rest = [1, 2, 3, 4];
        let expected_reg = 3;
        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_restore.0 | expected_reg)
            .append_bytes(&expected_rest);
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);
        assert_eq!(
            parse_cfi_instruction(input, 8),
            Ok(CallFrameInstruction::Restore {
                register: Register(expected_reg.into()),
            })
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_nop() {
        let expected_rest = [1, 2, 3, 4];
        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_nop.0)
            .append_bytes(&expected_rest);
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);
        assert_eq!(
            parse_cfi_instruction(input, 8),
            Ok(CallFrameInstruction::Nop)
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_set_loc() {
        let expected_rest = [1, 2, 3, 4];
        let expected_addr = 0xdead_beef;
        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_set_loc.0)
            .L64(expected_addr)
            .append_bytes(&expected_rest);
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);
        assert_eq!(
            parse_cfi_instruction(input, 8),
            Ok(CallFrameInstruction::SetLoc {
                address: expected_addr,
            })
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_set_loc_encoding() {
        let text_base = 0xfeed_face;
        let addr_offset = 0xbeef;
        let expected_addr = text_base + addr_offset;
        let expected_rest = [1, 2, 3, 4];
        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_set_loc.0)
            .L64(addr_offset)
            .append_bytes(&expected_rest);
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);
        let parameters = &PointerEncodingParameters {
            bases: &BaseAddresses::default().set_text(text_base).eh_frame,
            func_base: None,
            address_size: 8,
            section: &EndianSlice::new(&[], LittleEndian),
        };
        assert_eq!(
            CallFrameInstruction::parse(
                input,
                Some(constants::DW_EH_PE_textrel),
                parameters,
                Vendor::Default
            ),
            Ok(CallFrameInstruction::SetLoc {
                address: expected_addr,
            })
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_advance_loc1() {
        let expected_rest = [1, 2, 3, 4];
        let expected_delta = 8;
        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_advance_loc1.0)
            .D8(expected_delta)
            .append_bytes(&expected_rest);
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);
        assert_eq!(
            parse_cfi_instruction(input, 8),
            Ok(CallFrameInstruction::AdvanceLoc {
                delta: u32::from(expected_delta),
            })
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_advance_loc2() {
        let expected_rest = [1, 2, 3, 4];
        let expected_delta = 500;
        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_advance_loc2.0)
            .L16(expected_delta)
            .append_bytes(&expected_rest);
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);
        assert_eq!(
            parse_cfi_instruction(input, 8),
            Ok(CallFrameInstruction::AdvanceLoc {
                delta: u32::from(expected_delta),
            })
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_advance_loc4() {
        let expected_rest = [1, 2, 3, 4];
        let expected_delta = 1 << 20;
        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_advance_loc4.0)
            .L32(expected_delta)
            .append_bytes(&expected_rest);
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);
        assert_eq!(
            parse_cfi_instruction(input, 8),
            Ok(CallFrameInstruction::AdvanceLoc {
                delta: expected_delta,
            })
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_offset_extended() {
        let expected_rest = [1, 2, 3, 4];
        let expected_reg = 7;
        let expected_offset = 33;
        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_offset_extended.0)
            .uleb(expected_reg.into())
            .uleb(expected_offset)
            .append_bytes(&expected_rest);
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);
        assert_eq!(
            parse_cfi_instruction(input, 8),
            Ok(CallFrameInstruction::Offset {
                register: Register(expected_reg),
                factored_offset: expected_offset,
            })
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_restore_extended() {
        let expected_rest = [1, 2, 3, 4];
        let expected_reg = 7;
        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_restore_extended.0)
            .uleb(expected_reg.into())
            .append_bytes(&expected_rest);
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);
        assert_eq!(
            parse_cfi_instruction(input, 8),
            Ok(CallFrameInstruction::Restore {
                register: Register(expected_reg),
            })
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_undefined() {
        let expected_rest = [1, 2, 3, 4];
        let expected_reg = 7;
        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_undefined.0)
            .uleb(expected_reg.into())
            .append_bytes(&expected_rest);
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);
        assert_eq!(
            parse_cfi_instruction(input, 8),
            Ok(CallFrameInstruction::Undefined {
                register: Register(expected_reg),
            })
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_same_value() {
        let expected_rest = [1, 2, 3, 4];
        let expected_reg = 7;
        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_same_value.0)
            .uleb(expected_reg.into())
            .append_bytes(&expected_rest);
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);
        assert_eq!(
            parse_cfi_instruction(input, 8),
            Ok(CallFrameInstruction::SameValue {
                register: Register(expected_reg),
            })
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_register() {
        let expected_rest = [1, 2, 3, 4];
        let expected_dest_reg = 7;
        let expected_src_reg = 8;
        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_register.0)
            .uleb(expected_dest_reg.into())
            .uleb(expected_src_reg.into())
            .append_bytes(&expected_rest);
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);
        assert_eq!(
            parse_cfi_instruction(input, 8),
            Ok(CallFrameInstruction::Register {
                dest_register: Register(expected_dest_reg),
                src_register: Register(expected_src_reg),
            })
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_remember_state() {
        let expected_rest = [1, 2, 3, 4];
        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_remember_state.0)
            .append_bytes(&expected_rest);
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);
        assert_eq!(
            parse_cfi_instruction(input, 8),
            Ok(CallFrameInstruction::RememberState)
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_restore_state() {
        let expected_rest = [1, 2, 3, 4];
        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_restore_state.0)
            .append_bytes(&expected_rest);
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);
        assert_eq!(
            parse_cfi_instruction(input, 8),
            Ok(CallFrameInstruction::RestoreState)
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_def_cfa() {
        let expected_rest = [1, 2, 3, 4];
        let expected_reg = 2;
        let expected_offset = 0;
        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_def_cfa.0)
            .uleb(expected_reg.into())
            .uleb(expected_offset)
            .append_bytes(&expected_rest);
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);
        assert_eq!(
            parse_cfi_instruction(input, 8),
            Ok(CallFrameInstruction::DefCfa {
                register: Register(expected_reg),
                offset: expected_offset,
            })
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_def_cfa_register() {
        let expected_rest = [1, 2, 3, 4];
        let expected_reg = 2;
        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_def_cfa_register.0)
            .uleb(expected_reg.into())
            .append_bytes(&expected_rest);
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);
        assert_eq!(
            parse_cfi_instruction(input, 8),
            Ok(CallFrameInstruction::DefCfaRegister {
                register: Register(expected_reg),
            })
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_def_cfa_offset() {
        let expected_rest = [1, 2, 3, 4];
        let expected_offset = 23;
        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_def_cfa_offset.0)
            .uleb(expected_offset)
            .append_bytes(&expected_rest);
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);
        assert_eq!(
            parse_cfi_instruction(input, 8),
            Ok(CallFrameInstruction::DefCfaOffset {
                offset: expected_offset,
            })
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_def_cfa_expression() {
        let expected_rest = [1, 2, 3, 4];
        let expected_expr = [10, 9, 8, 7, 6, 5, 4, 3, 2, 1];

        let length = Label::new();
        let start = Label::new();
        let end = Label::new();

        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_def_cfa_expression.0)
            .D8(&length)
            .mark(&start)
            .append_bytes(&expected_expr)
            .mark(&end)
            .append_bytes(&expected_rest);

        length.set_const((&end - &start) as u64);
        let expected_expression = UnwindExpression {
            offset: (&start - &section.start()) as usize,
            length: (&end - &start) as usize,
        };
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);

        assert_eq!(
            parse_cfi_instruction(input, 8),
            Ok(CallFrameInstruction::DefCfaExpression {
                expression: expected_expression,
            })
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_expression() {
        let expected_rest = [1, 2, 3, 4];
        let expected_reg = 99;
        let expected_expr = [10, 9, 8, 7, 6, 5, 4, 3, 2, 1];

        let length = Label::new();
        let start = Label::new();
        let end = Label::new();

        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_expression.0)
            .uleb(expected_reg.into())
            .D8(&length)
            .mark(&start)
            .append_bytes(&expected_expr)
            .mark(&end)
            .append_bytes(&expected_rest);

        length.set_const((&end - &start) as u64);
        let expected_expression = UnwindExpression {
            offset: (&start - &section.start()) as usize,
            length: (&end - &start) as usize,
        };
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);

        assert_eq!(
            parse_cfi_instruction(input, 8),
            Ok(CallFrameInstruction::Expression {
                register: Register(expected_reg),
                expression: expected_expression,
            })
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_offset_extended_sf() {
        let expected_rest = [1, 2, 3, 4];
        let expected_reg = 7;
        let expected_offset = -33;
        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_offset_extended_sf.0)
            .uleb(expected_reg.into())
            .sleb(expected_offset)
            .append_bytes(&expected_rest);
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);
        assert_eq!(
            parse_cfi_instruction(input, 8),
            Ok(CallFrameInstruction::OffsetExtendedSf {
                register: Register(expected_reg),
                factored_offset: expected_offset,
            })
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_def_cfa_sf() {
        let expected_rest = [1, 2, 3, 4];
        let expected_reg = 2;
        let expected_offset = -9999;
        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_def_cfa_sf.0)
            .uleb(expected_reg.into())
            .sleb(expected_offset)
            .append_bytes(&expected_rest);
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);
        assert_eq!(
            parse_cfi_instruction(input, 8),
            Ok(CallFrameInstruction::DefCfaSf {
                register: Register(expected_reg),
                factored_offset: expected_offset,
            })
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_def_cfa_offset_sf() {
        let expected_rest = [1, 2, 3, 4];
        let expected_offset = -123;
        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_def_cfa_offset_sf.0)
            .sleb(expected_offset)
            .append_bytes(&expected_rest);
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);
        assert_eq!(
            parse_cfi_instruction(input, 8),
            Ok(CallFrameInstruction::DefCfaOffsetSf {
                factored_offset: expected_offset,
            })
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_val_offset() {
        let expected_rest = [1, 2, 3, 4];
        let expected_reg = 50;
        let expected_offset = 23;
        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_val_offset.0)
            .uleb(expected_reg.into())
            .uleb(expected_offset)
            .append_bytes(&expected_rest);
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);
        assert_eq!(
            parse_cfi_instruction(input, 8),
            Ok(CallFrameInstruction::ValOffset {
                register: Register(expected_reg),
                factored_offset: expected_offset,
            })
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_val_offset_sf() {
        let expected_rest = [1, 2, 3, 4];
        let expected_reg = 50;
        let expected_offset = -23;
        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_val_offset_sf.0)
            .uleb(expected_reg.into())
            .sleb(expected_offset)
            .append_bytes(&expected_rest);
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);
        assert_eq!(
            parse_cfi_instruction(input, 8),
            Ok(CallFrameInstruction::ValOffsetSf {
                register: Register(expected_reg),
                factored_offset: expected_offset,
            })
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_val_expression() {
        let expected_rest = [1, 2, 3, 4];
        let expected_reg = 50;
        let expected_expr = [2, 2, 1, 1, 5, 5];

        let length = Label::new();
        let start = Label::new();
        let end = Label::new();

        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_val_expression.0)
            .uleb(expected_reg.into())
            .D8(&length)
            .mark(&start)
            .append_bytes(&expected_expr)
            .mark(&end)
            .append_bytes(&expected_rest);

        length.set_const((&end - &start) as u64);
        let expected_expression = UnwindExpression {
            offset: (&start - &section.start()) as usize,
            length: (&end - &start) as usize,
        };
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);

        assert_eq!(
            parse_cfi_instruction(input, 8),
            Ok(CallFrameInstruction::ValExpression {
                register: Register(expected_reg),
                expression: expected_expression,
            })
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_negate_ra_state() {
        let expected_rest = [1, 2, 3, 4];
        let section = Section::with_endian(Endian::Little)
            .D8(constants::DW_CFA_AARCH64_negate_ra_state.0)
            .append_bytes(&expected_rest);
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);
        let parameters = &PointerEncodingParameters {
            bases: &SectionBaseAddresses::default(),
            func_base: None,
            address_size: 8,
            section: &EndianSlice::default(),
        };
        assert_eq!(
            CallFrameInstruction::parse(input, None, parameters, Vendor::AArch64),
            Ok(CallFrameInstruction::NegateRaState)
        );
        assert_eq!(*input, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_cfi_instruction_unknown_instruction() {
        let expected_rest = [1, 2, 3, 4];
        let unknown_instr = constants::DwCfa(0b0011_1111);
        let section = Section::with_endian(Endian::Little)
            .D8(unknown_instr.0)
            .append_bytes(&expected_rest);
        let contents = section.get_contents().unwrap();
        let input = &mut EndianSlice::new(&contents, LittleEndian);
        assert_eq!(
            parse_cfi_instruction(input, 8),
            Err(Error::UnknownCallFrameInstruction(unknown_instr))
        );
    }

    #[test]
    fn test_call_frame_instruction_iter_ok() {
        let expected_reg = 50;
        let expected_expr = [2, 2, 1, 1, 5, 5];
        let expected_delta = 230;

        let length = Label::new();
        let start = Label::new();
        let end = Label::new();

        let section = Section::with_endian(Endian::Big)
            .D8(constants::DW_CFA_val_expression.0)
            .uleb(expected_reg.into())
            .D8(&length)
            .mark(&start)
            .append_bytes(&expected_expr)
            .mark(&end)
            .D8(constants::DW_CFA_advance_loc1.0)
            .D8(expected_delta);

        length.set_const((&end - &start) as u64);
        let expected_expression = UnwindExpression {
            offset: (&start - &section.start()) as usize,
            length: (&end - &start) as usize,
        };
        let contents = section.get_contents().unwrap();
        let input = EndianSlice::new(&contents, BigEndian);
        let parameters = PointerEncodingParameters {
            bases: &SectionBaseAddresses::default(),
            func_base: None,
            address_size: 8,
            section: &input,
        };
        let mut iter = CallFrameInstructionIter {
            input,
            address_encoding: None,
            parameters,
            vendor: Vendor::Default,
        };

        assert_eq!(
            iter.next(),
            Ok(Some(CallFrameInstruction::ValExpression {
                register: Register(expected_reg),
                expression: expected_expression,
            }))
        );

        assert_eq!(
            iter.next(),
            Ok(Some(CallFrameInstruction::AdvanceLoc {
                delta: u32::from(expected_delta),
            }))
        );

        assert_eq!(iter.next(), Ok(None));
    }

    #[test]
    fn test_call_frame_instruction_iter_err() {
        // DW_CFA_advance_loc1 without an operand.
        let section = Section::with_endian(Endian::Big).D8(constants::DW_CFA_advance_loc1.0);

        let contents = section.get_contents().unwrap();
        let input = EndianSlice::new(&contents, BigEndian);
        let parameters = PointerEncodingParameters {
            bases: &SectionBaseAddresses::default(),
            func_base: None,
            address_size: 8,
            section: &EndianSlice::default(),
        };
        let mut iter = CallFrameInstructionIter {
            input,
            address_encoding: None,
            parameters,
            vendor: Vendor::Default,
        };

        assert_eq!(
            iter.next().map_eof(&contents),
            Err(Error::UnexpectedEof(ReaderOffsetId(1)))
        );
        assert_eq!(iter.next(), Ok(None));
    }

    fn assert_eval<'a, I>(
        mut initial_ctx: UnwindContext<usize>,
        expected_ctx: UnwindContext<usize>,
        cie: CommonInformationEntry<EndianSlice<'a, LittleEndian>>,
        fde: Option<FrameDescriptionEntry<EndianSlice<'a, LittleEndian>>>,
        instructions: I,
    ) where
        I: AsRef<[(Result<bool>, CallFrameInstruction<usize>)]>,
    {
        {
            let section = &DebugFrame::from(EndianSlice::default());
            let bases = &BaseAddresses::default();
            let mut table = match fde {
                Some(fde) => UnwindTable::new_for_fde(section, bases, &mut initial_ctx, &fde),
                None => UnwindTable::new_for_cie(section, bases, &mut initial_ctx, &cie),
            };
            for (expected_result, instruction) in instructions.as_ref() {
                assert_eq!(*expected_result, table.evaluate(instruction.clone()));
            }
        }

        assert_eq!(expected_ctx, initial_ctx);
    }

    fn make_test_cie<'a>() -> CommonInformationEntry<EndianSlice<'a, LittleEndian>> {
        CommonInformationEntry {
            offset: 0,
            format: Format::Dwarf64,
            length: 0,
            return_address_register: Register(0),
            version: 4,
            address_size: mem::size_of::<usize>() as u8,
            initial_instructions: EndianSlice::new(&[], LittleEndian),
            augmentation: None,
            data_alignment_factor: 2,
            code_alignment_factor: 3,
        }
    }

    #[test]
    fn test_eval_set_loc() {
        let cie = make_test_cie();
        let ctx = UnwindContext::new();
        let mut expected = ctx.clone();
        expected.row_mut().end_address = 42;
        let instructions = [(Ok(true), CallFrameInstruction::SetLoc { address: 42 })];
        assert_eval(ctx, expected, cie, None, instructions);
    }

    #[test]
    fn test_eval_set_loc_backwards() {
        let cie = make_test_cie();
        let mut ctx = UnwindContext::new();
        ctx.row_mut().start_address = 999;
        let expected = ctx.clone();
        let instructions = [(
            Err(Error::InvalidAddressRange),
            CallFrameInstruction::SetLoc { address: 42 },
        )];
        assert_eval(ctx, expected, cie, None, instructions);
    }

    #[test]
    fn test_eval_advance_loc() {
        let cie = make_test_cie();
        let mut ctx = UnwindContext::new();
        ctx.row_mut().start_address = 3;
        let mut expected = ctx.clone();
        expected.row_mut().end_address = 3 + 2 * cie.code_alignment_factor;
        let instructions = [(Ok(true), CallFrameInstruction::AdvanceLoc { delta: 2 })];
        assert_eval(ctx, expected, cie, None, instructions);
    }

    #[test]
    fn test_eval_advance_loc_overflow_32() {
        let mut cie = make_test_cie();
        cie.address_size = 4;
        let mut ctx = UnwindContext::new();
        ctx.row_mut().start_address = u32::MAX.into();
        let expected = ctx.clone();
        let instructions = [(
            Err(Error::AddressOverflow),
            CallFrameInstruction::AdvanceLoc { delta: 42 },
        )];
        assert_eval(ctx, expected, cie, None, instructions);
    }

    #[test]
    fn test_eval_advance_loc_overflow_64() {
        let mut cie = make_test_cie();
        cie.address_size = 8;
        let mut ctx = UnwindContext::new();
        ctx.row_mut().start_address = u64::MAX;
        let expected = ctx.clone();
        let instructions = [(
            Err(Error::AddressOverflow),
            CallFrameInstruction::AdvanceLoc { delta: 42 },
        )];
        assert_eval(ctx, expected, cie, None, instructions);
    }

    #[test]
    fn test_eval_def_cfa() {
        let cie = make_test_cie();
        let ctx = UnwindContext::new();
        let mut expected = ctx.clone();
        expected.set_cfa(CfaRule::RegisterAndOffset {
            register: Register(42),
            offset: 36,
        });
        let instructions = [(
            Ok(false),
            CallFrameInstruction::DefCfa {
                register: Register(42),
                offset: 36,
            },
        )];
        assert_eval(ctx, expected, cie, None, instructions);
    }

    #[test]
    fn test_eval_def_cfa_sf() {
        let cie = make_test_cie();
        let ctx = UnwindContext::new();
        let mut expected = ctx.clone();
        expected.set_cfa(CfaRule::RegisterAndOffset {
            register: Register(42),
            offset: 36 * cie.data_alignment_factor as i64,
        });
        let instructions = [(
            Ok(false),
            CallFrameInstruction::DefCfaSf {
                register: Register(42),
                factored_offset: 36,
            },
        )];
        assert_eval(ctx, expected, cie, None, instructions);
    }

    #[test]
    fn test_eval_def_cfa_register() {
        let cie = make_test_cie();
        let mut ctx = UnwindContext::new();
        ctx.set_cfa(CfaRule::RegisterAndOffset {
            register: Register(3),
            offset: 8,
        });
        let mut expected = ctx.clone();
        expected.set_cfa(CfaRule::RegisterAndOffset {
            register: Register(42),
            offset: 8,
        });
        let instructions = [(
            Ok(false),
            CallFrameInstruction::DefCfaRegister {
                register: Register(42),
            },
        )];
        assert_eval(ctx, expected, cie, None, instructions);
    }

    #[test]
    fn test_eval_def_cfa_register_invalid_context() {
        let cie = make_test_cie();
        let mut ctx = UnwindContext::new();
        ctx.set_cfa(CfaRule::Expression(UnwindExpression {
            offset: 0,
            length: 0,
        }));
        let expected = ctx.clone();
        let instructions = [(
            Err(Error::CfiInstructionInInvalidContext),
            CallFrameInstruction::DefCfaRegister {
                register: Register(42),
            },
        )];
        assert_eval(ctx, expected, cie, None, instructions);
    }

    #[test]
    fn test_eval_def_cfa_offset() {
        let cie = make_test_cie();
        let mut ctx = UnwindContext::new();
        ctx.set_cfa(CfaRule::RegisterAndOffset {
            register: Register(3),
            offset: 8,
        });
        let mut expected = ctx.clone();
        expected.set_cfa(CfaRule::RegisterAndOffset {
            register: Register(3),
            offset: 42,
        });
        let instructions = [(Ok(false), CallFrameInstruction::DefCfaOffset { offset: 42 })];
        assert_eval(ctx, expected, cie, None, instructions);
    }

    #[test]
    fn test_eval_def_cfa_offset_invalid_context() {
        let cie = make_test_cie();
        let mut ctx = UnwindContext::new();
        ctx.set_cfa(CfaRule::Expression(UnwindExpression {
            offset: 10,
            length: 11,
        }));
        let expected = ctx.clone();
        let instructions = [(
            Err(Error::CfiInstructionInInvalidContext),
            CallFrameInstruction::DefCfaOffset { offset: 1993 },
        )];
        assert_eval(ctx, expected, cie, None, instructions);
    }

    #[test]
    fn test_eval_def_cfa_expression() {
        let expr = UnwindExpression {
            offset: 10,
            length: 11,
        };
        let cie = make_test_cie();
        let ctx = UnwindContext::new();
        let mut expected = ctx.clone();
        expected.set_cfa(CfaRule::Expression(expr));
        let instructions = [(
            Ok(false),
            CallFrameInstruction::DefCfaExpression { expression: expr },
        )];
        assert_eval(ctx, expected, cie, None, instructions);
    }

    #[test]
    fn test_eval_undefined() {
        let cie = make_test_cie();
        let ctx = UnwindContext::new();
        let mut expected = ctx.clone();
        expected
            .set_register_rule(Register(5), RegisterRule::Undefined)
            .unwrap();
        let instructions = [(
            Ok(false),
            CallFrameInstruction::Undefined {
                register: Register(5),
            },
        )];
        assert_eval(ctx, expected, cie, None, instructions);
    }

    #[test]
    fn test_eval_same_value() {
        let cie = make_test_cie();
        let ctx = UnwindContext::new();
        let mut expected = ctx.clone();
        expected
            .set_register_rule(Register(0), RegisterRule::SameValue)
            .unwrap();
        let instructions = [(
            Ok(false),
            CallFrameInstruction::SameValue {
                register: Register(0),
            },
        )];
        assert_eval(ctx, expected, cie, None, instructions);
    }

    #[test]
    fn test_eval_offset() {
        let cie = make_test_cie();
        let ctx = UnwindContext::new();
        let mut expected = ctx.clone();
        expected
            .set_register_rule(
                Register(2),
                RegisterRule::Offset(3 * cie.data_alignment_factor),
            )
            .unwrap();
        let instructions = [(
            Ok(false),
            CallFrameInstruction::Offset {
                register: Register(2),
                factored_offset: 3,
            },
        )];
        assert_eval(ctx, expected, cie, None, instructions);
    }

    #[test]
    fn test_eval_offset_extended_sf() {
        let cie = make_test_cie();
        let ctx = UnwindContext::new();
        let mut expected = ctx.clone();
        expected
            .set_register_rule(
                Register(4),
                RegisterRule::Offset(-3 * cie.data_alignment_factor),
            )
            .unwrap();
        let instructions = [(
            Ok(false),
            CallFrameInstruction::OffsetExtendedSf {
                register: Register(4),
                factored_offset: -3,
            },
        )];
        assert_eval(ctx, expected, cie, None, instructions);
    }

    #[test]
    fn test_eval_val_offset() {
        let cie = make_test_cie();
        let ctx = UnwindContext::new();
        let mut expected = ctx.clone();
        expected
            .set_register_rule(
                Register(5),
                RegisterRule::ValOffset(7 * cie.data_alignment_factor),
            )
            .unwrap();
        let instructions = [(
            Ok(false),
            CallFrameInstruction::ValOffset {
                register: Register(5),
                factored_offset: 7,
            },
        )];
        assert_eval(ctx, expected, cie, None, instructions);
    }

    #[test]
    fn test_eval_val_offset_sf() {
        let cie = make_test_cie();
        let ctx = UnwindContext::new();
        let mut expected = ctx.clone();
        expected
            .set_register_rule(
                Register(5),
                RegisterRule::ValOffset(-7 * cie.data_alignment_factor),
            )
            .unwrap();
        let instructions = [(
            Ok(false),
            CallFrameInstruction::ValOffsetSf {
                register: Register(5),
                factored_offset: -7,
            },
        )];
        assert_eval(ctx, expected, cie, None, instructions);
    }

    #[test]
    fn test_eval_expression() {
        let expr = UnwindExpression {
            offset: 10,
            length: 11,
        };
        let cie = make_test_cie();
        let ctx = UnwindContext::new();
        let mut expected = ctx.clone();
        expected
            .set_register_rule(Register(9), RegisterRule::Expression(expr))
            .unwrap();
        let instructions = [(
            Ok(false),
            CallFrameInstruction::Expression {
                register: Register(9),
                expression: expr,
            },
        )];
        assert_eval(ctx, expected, cie, None, instructions);
    }

    #[test]
    fn test_eval_val_expression() {
        let expr = UnwindExpression {
            offset: 10,
            length: 11,
        };
        let cie = make_test_cie();
        let ctx = UnwindContext::new();
        let mut expected = ctx.clone();
        expected
            .set_register_rule(Register(9), RegisterRule::ValExpression(expr))
            .unwrap();
        let instructions = [(
            Ok(false),
            CallFrameInstruction::ValExpression {
                register: Register(9),
                expression: expr,
            },
        )];
        assert_eval(ctx, expected, cie, None, instructions);
    }

    #[test]
    fn test_eval_restore() {
        let cie = make_test_cie();
        let fde = FrameDescriptionEntry {
            offset: 0,
            format: Format::Dwarf64,
            length: 0,
            address_range: 0,
            augmentation: None,
            initial_address: 0,
            cie: cie.clone(),
            instructions: EndianSlice::new(&[], LittleEndian),
        };

        let mut ctx = UnwindContext::new();
        ctx.set_register_rule(Register(0), RegisterRule::Offset(1))
            .unwrap();
        ctx.save_initial_rules().unwrap();
        let expected = ctx.clone();
        ctx.set_register_rule(Register(0), RegisterRule::Offset(2))
            .unwrap();

        let instructions = [(
            Ok(false),
            CallFrameInstruction::Restore {
                register: Register(0),
            },
        )];
        assert_eval(ctx, expected, cie, Some(fde), instructions);
    }

    #[test]
    fn test_eval_restore_havent_saved_initial_context() {
        let cie = make_test_cie();
        let ctx = UnwindContext::new();
        let expected = ctx.clone();
        let instructions = [(
            Err(Error::CfiInstructionInInvalidContext),
            CallFrameInstruction::Restore {
                register: Register(0),
            },
        )];
        assert_eval(ctx, expected, cie, None, instructions);
    }

    #[test]
    fn test_eval_remember_state() {
        let cie = make_test_cie();
        let ctx = UnwindContext::new();
        let mut expected = ctx.clone();
        expected.push_row().unwrap();
        let instructions = [(Ok(false), CallFrameInstruction::RememberState)];
        assert_eval(ctx, expected, cie, None, instructions);
    }

    #[test]
    fn test_eval_restore_state() {
        let cie = make_test_cie();

        let mut ctx = UnwindContext::new();
        ctx.set_start_address(1);
        ctx.set_register_rule(Register(0), RegisterRule::SameValue)
            .unwrap();
        let mut expected = ctx.clone();
        ctx.push_row().unwrap();
        ctx.set_start_address(2);
        ctx.set_register_rule(Register(0), RegisterRule::Offset(16))
            .unwrap();

        // Restore state should preserve current location.
        expected.set_start_address(2);

        let instructions = [
            // First one pops just fine.
            (Ok(false), CallFrameInstruction::RestoreState),
            // Second pop would try to pop out of bounds.
            (
                Err(Error::PopWithEmptyStack),
                CallFrameInstruction::RestoreState,
            ),
        ];

        assert_eval(ctx, expected, cie, None, instructions);
    }

    #[test]
    fn test_eval_negate_ra_state() {
        let cie = make_test_cie();
        let ctx = UnwindContext::new();
        let mut expected = ctx.clone();
        expected
            .set_register_rule(crate::AArch64::RA_SIGN_STATE, RegisterRule::Constant(1))
            .unwrap();
        let instructions = [(Ok(false), CallFrameInstruction::NegateRaState)];
        assert_eval(ctx, expected, cie, None, instructions);

        let cie = make_test_cie();
        let ctx = UnwindContext::new();
        let mut expected = ctx.clone();
        expected
            .set_register_rule(crate::AArch64::RA_SIGN_STATE, RegisterRule::Constant(0))
            .unwrap();
        let instructions = [
            (Ok(false), CallFrameInstruction::NegateRaState),
            (Ok(false), CallFrameInstruction::NegateRaState),
        ];
        assert_eval(ctx, expected, cie, None, instructions);

        // NegateRaState can't be used with other instructions.
        let cie = make_test_cie();
        let ctx = UnwindContext::new();
        let mut expected = ctx.clone();
        expected
            .set_register_rule(
                crate::AArch64::RA_SIGN_STATE,
                RegisterRule::Offset(cie.data_alignment_factor as i64),
            )
            .unwrap();
        let instructions = [
            (
                Ok(false),
                CallFrameInstruction::Offset {
                    register: crate::AArch64::RA_SIGN_STATE,
                    factored_offset: 1,
                },
            ),
            (
                Err(Error::CfiInstructionInInvalidContext),
                CallFrameInstruction::NegateRaState,
            ),
        ];
        assert_eval(ctx, expected, cie, None, instructions);
    }

    #[test]
    fn test_eval_nop() {
        let cie = make_test_cie();
        let ctx = UnwindContext::new();
        let expected = ctx.clone();
        let instructions = [(Ok(false), CallFrameInstruction::Nop)];
        assert_eval(ctx, expected, cie, None, instructions);
    }

    #[test]
    fn test_unwind_table_cie_no_rule() {
        let initial_instructions = Section::with_endian(Endian::Little)
            // The CFA is -12 from register 4.
            .D8(constants::DW_CFA_def_cfa_sf.0)
            .uleb(4)
            .sleb(-12)
            .append_repeated(constants::DW_CFA_nop.0, 4);
        let initial_instructions = initial_instructions.get_contents().unwrap();

        let cie = CommonInformationEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            version: 4,
            augmentation: None,
            address_size: 8,
            code_alignment_factor: 1,
            data_alignment_factor: 1,
            return_address_register: Register(3),
            initial_instructions: EndianSlice::new(&initial_instructions, LittleEndian),
        };

        let instructions = Section::with_endian(Endian::Little)
            // A bunch of nop padding.
            .append_repeated(constants::DW_CFA_nop.0, 8);
        let instructions = instructions.get_contents().unwrap();

        let fde = FrameDescriptionEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            cie: cie.clone(),
            initial_address: 0,
            address_range: 100,
            augmentation: None,
            instructions: EndianSlice::new(&instructions, LittleEndian),
        };

        let section = &DebugFrame::from(EndianSlice::default());
        let bases = &BaseAddresses::default();
        let mut ctx = Box::new(UnwindContext::new());

        let mut table = fde
            .rows(section, bases, &mut ctx)
            .expect("Should run initial program OK");
        assert!(table.ctx.is_initialized);
        let expected_initial_rule = (Register(0), RegisterRule::Undefined);
        assert_eq!(table.ctx.initial_rule, Some(expected_initial_rule));

        {
            let row = table.next_row().expect("Should evaluate first row OK");
            let expected = UnwindTableRow {
                start_address: 0,
                end_address: 100,
                saved_args_size: 0,
                cfa: CfaRule::RegisterAndOffset {
                    register: Register(4),
                    offset: -12,
                },
                registers: [].iter().collect(),
            };
            assert_eq!(Some(&expected), row);
        }

        // All done!
        assert_eq!(Ok(None), table.next_row());
        assert_eq!(Ok(None), table.next_row());
    }

    #[test]
    fn test_unwind_table_cie_single_rule() {
        let initial_instructions = Section::with_endian(Endian::Little)
            // The CFA is -12 from register 4.
            .D8(constants::DW_CFA_def_cfa_sf.0)
            .uleb(4)
            .sleb(-12)
            // Register 3 is 4 from the CFA.
            .D8(constants::DW_CFA_offset.0 | 3)
            .uleb(4)
            .append_repeated(constants::DW_CFA_nop.0, 4);
        let initial_instructions = initial_instructions.get_contents().unwrap();

        let cie = CommonInformationEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            version: 4,
            augmentation: None,
            address_size: 8,
            code_alignment_factor: 1,
            data_alignment_factor: 1,
            return_address_register: Register(3),
            initial_instructions: EndianSlice::new(&initial_instructions, LittleEndian),
        };

        let instructions = Section::with_endian(Endian::Little)
            // A bunch of nop padding.
            .append_repeated(constants::DW_CFA_nop.0, 8);
        let instructions = instructions.get_contents().unwrap();

        let fde = FrameDescriptionEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            cie: cie.clone(),
            initial_address: 0,
            address_range: 100,
            augmentation: None,
            instructions: EndianSlice::new(&instructions, LittleEndian),
        };

        let section = &DebugFrame::from(EndianSlice::default());
        let bases = &BaseAddresses::default();
        let mut ctx = Box::new(UnwindContext::new());

        let mut table = fde
            .rows(section, bases, &mut ctx)
            .expect("Should run initial program OK");
        assert!(table.ctx.is_initialized);
        let expected_initial_rule = (Register(3), RegisterRule::Offset(4));
        assert_eq!(table.ctx.initial_rule, Some(expected_initial_rule));

        {
            let row = table.next_row().expect("Should evaluate first row OK");
            let expected = UnwindTableRow {
                start_address: 0,
                end_address: 100,
                saved_args_size: 0,
                cfa: CfaRule::RegisterAndOffset {
                    register: Register(4),
                    offset: -12,
                },
                registers: [(Register(3), RegisterRule::Offset(4))].iter().collect(),
            };
            assert_eq!(Some(&expected), row);
        }

        // All done!
        assert_eq!(Ok(None), table.next_row());
        assert_eq!(Ok(None), table.next_row());
    }

    #[test]
    fn test_unwind_table_cie_invalid_rule() {
        let initial_instructions1 = Section::with_endian(Endian::Little)
            // Test that stack length is reset.
            .D8(constants::DW_CFA_remember_state.0)
            // Test that stack value is reset (different register from that used later).
            .D8(constants::DW_CFA_offset.0 | 4)
            .uleb(8)
            // Invalid due to missing operands.
            .D8(constants::DW_CFA_offset.0);
        let initial_instructions1 = initial_instructions1.get_contents().unwrap();

        let cie1 = CommonInformationEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            version: 4,
            augmentation: None,
            address_size: 8,
            code_alignment_factor: 1,
            data_alignment_factor: 1,
            return_address_register: Register(3),
            initial_instructions: EndianSlice::new(&initial_instructions1, LittleEndian),
        };

        let initial_instructions2 = Section::with_endian(Endian::Little)
            // Register 3 is 4 from the CFA.
            .D8(constants::DW_CFA_offset.0 | 3)
            .uleb(4)
            .append_repeated(constants::DW_CFA_nop.0, 4);
        let initial_instructions2 = initial_instructions2.get_contents().unwrap();

        let cie2 = CommonInformationEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            version: 4,
            augmentation: None,
            address_size: 8,
            code_alignment_factor: 1,
            data_alignment_factor: 1,
            return_address_register: Register(3),
            initial_instructions: EndianSlice::new(&initial_instructions2, LittleEndian),
        };

        let fde1 = FrameDescriptionEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            cie: cie1.clone(),
            initial_address: 0,
            address_range: 100,
            augmentation: None,
            instructions: EndianSlice::new(&[], LittleEndian),
        };

        let fde2 = FrameDescriptionEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            cie: cie2.clone(),
            initial_address: 0,
            address_range: 100,
            augmentation: None,
            instructions: EndianSlice::new(&[], LittleEndian),
        };

        let section = &DebugFrame::from(EndianSlice::default());
        let bases = &BaseAddresses::default();
        let mut ctx = Box::new(UnwindContext::new());

        let table = fde1
            .rows(section, bases, &mut ctx)
            .map_eof(&initial_instructions1);
        assert_eq!(table.err(), Some(Error::UnexpectedEof(ReaderOffsetId(4))));
        assert!(!ctx.is_initialized);
        assert_eq!(ctx.stack.len(), 2);
        assert_eq!(ctx.initial_rule, None);

        let _table = fde2
            .rows(section, bases, &mut ctx)
            .expect("Should run initial program OK");
        assert!(ctx.is_initialized);
        assert_eq!(ctx.stack.len(), 1);
        let expected_initial_rule = (Register(3), RegisterRule::Offset(4));
        assert_eq!(ctx.initial_rule, Some(expected_initial_rule));
    }

    #[test]
    fn test_unwind_table_next_row() {
        #[allow(clippy::identity_op)]
        let initial_instructions = Section::with_endian(Endian::Little)
            // The CFA is -12 from register 4.
            .D8(constants::DW_CFA_def_cfa_sf.0)
            .uleb(4)
            .sleb(-12)
            // Register 0 is 8 from the CFA.
            .D8(constants::DW_CFA_offset.0 | 0)
            .uleb(8)
            // Register 3 is 4 from the CFA.
            .D8(constants::DW_CFA_offset.0 | 3)
            .uleb(4)
            .append_repeated(constants::DW_CFA_nop.0, 4);
        let initial_instructions = initial_instructions.get_contents().unwrap();

        let cie = CommonInformationEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            version: 4,
            augmentation: None,
            address_size: 8,
            code_alignment_factor: 1,
            data_alignment_factor: 1,
            return_address_register: Register(3),
            initial_instructions: EndianSlice::new(&initial_instructions, LittleEndian),
        };

        let instructions = Section::with_endian(Endian::Little)
            // Initial instructions form a row, advance the address by 1.
            .D8(constants::DW_CFA_advance_loc1.0)
            .D8(1)
            // Register 0 is -16 from the CFA.
            .D8(constants::DW_CFA_offset_extended_sf.0)
            .uleb(0)
            .sleb(-16)
            // Finish this row, advance the address by 32.
            .D8(constants::DW_CFA_advance_loc1.0)
            .D8(32)
            // Register 3 is -4 from the CFA.
            .D8(constants::DW_CFA_offset_extended_sf.0)
            .uleb(3)
            .sleb(-4)
            // Finish this row, advance the address by 64.
            .D8(constants::DW_CFA_advance_loc1.0)
            .D8(64)
            // Register 5 is 4 from the CFA.
            .D8(constants::DW_CFA_offset.0 | 5)
            .uleb(4)
            // A bunch of nop padding.
            .append_repeated(constants::DW_CFA_nop.0, 8);
        let instructions = instructions.get_contents().unwrap();

        let fde = FrameDescriptionEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            cie: cie.clone(),
            initial_address: 0,
            address_range: 100,
            augmentation: None,
            instructions: EndianSlice::new(&instructions, LittleEndian),
        };

        let section = &DebugFrame::from(EndianSlice::default());
        let bases = &BaseAddresses::default();
        let mut ctx = Box::new(UnwindContext::new());

        let mut table = fde
            .rows(section, bases, &mut ctx)
            .expect("Should run initial program OK");
        assert!(table.ctx.is_initialized);
        assert!(table.ctx.initial_rule.is_none());
        let expected_initial_rules: RegisterRuleMap<_> = [
            (Register(0), RegisterRule::Offset(8)),
            (Register(3), RegisterRule::Offset(4)),
        ]
        .iter()
        .collect();
        assert_eq!(table.ctx.stack[0].registers, expected_initial_rules);

        {
            let row = table.next_row().expect("Should evaluate first row OK");
            let expected = UnwindTableRow {
                start_address: 0,
                end_address: 1,
                saved_args_size: 0,
                cfa: CfaRule::RegisterAndOffset {
                    register: Register(4),
                    offset: -12,
                },
                registers: [
                    (Register(0), RegisterRule::Offset(8)),
                    (Register(3), RegisterRule::Offset(4)),
                ]
                .iter()
                .collect(),
            };
            assert_eq!(Some(&expected), row);
        }

        {
            let row = table.next_row().expect("Should evaluate second row OK");
            let expected = UnwindTableRow {
                start_address: 1,
                end_address: 33,
                saved_args_size: 0,
                cfa: CfaRule::RegisterAndOffset {
                    register: Register(4),
                    offset: -12,
                },
                registers: [
                    (Register(0), RegisterRule::Offset(-16)),
                    (Register(3), RegisterRule::Offset(4)),
                ]
                .iter()
                .collect(),
            };
            assert_eq!(Some(&expected), row);
        }

        {
            let row = table.next_row().expect("Should evaluate third row OK");
            let expected = UnwindTableRow {
                start_address: 33,
                end_address: 97,
                saved_args_size: 0,
                cfa: CfaRule::RegisterAndOffset {
                    register: Register(4),
                    offset: -12,
                },
                registers: [
                    (Register(0), RegisterRule::Offset(-16)),
                    (Register(3), RegisterRule::Offset(-4)),
                ]
                .iter()
                .collect(),
            };
            assert_eq!(Some(&expected), row);
        }

        {
            let row = table.next_row().expect("Should evaluate fourth row OK");
            let expected = UnwindTableRow {
                start_address: 97,
                end_address: 100,
                saved_args_size: 0,
                cfa: CfaRule::RegisterAndOffset {
                    register: Register(4),
                    offset: -12,
                },
                registers: [
                    (Register(0), RegisterRule::Offset(-16)),
                    (Register(3), RegisterRule::Offset(-4)),
                    (Register(5), RegisterRule::Offset(4)),
                ]
                .iter()
                .collect(),
            };
            assert_eq!(Some(&expected), row);
        }

        // All done!
        assert_eq!(Ok(None), table.next_row());
        assert_eq!(Ok(None), table.next_row());
    }

    #[test]
    fn test_unwind_info_for_address_ok() {
        let instrs1 = Section::with_endian(Endian::Big)
            // The CFA is -12 from register 4.
            .D8(constants::DW_CFA_def_cfa_sf.0)
            .uleb(4)
            .sleb(-12);
        let instrs1 = instrs1.get_contents().unwrap();

        let instrs2: Vec<_> = (0..8).map(|_| constants::DW_CFA_nop.0).collect();

        let instrs3 = Section::with_endian(Endian::Big)
            // Initial instructions form a row, advance the address by 100.
            .D8(constants::DW_CFA_advance_loc1.0)
            .D8(100)
            // Register 0 is -16 from the CFA.
            .D8(constants::DW_CFA_offset_extended_sf.0)
            .uleb(0)
            .sleb(-16);
        let instrs3 = instrs3.get_contents().unwrap();

        let instrs4: Vec<_> = (0..16).map(|_| constants::DW_CFA_nop.0).collect();

        let mut cie1 = CommonInformationEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            version: 4,
            augmentation: None,
            address_size: 8,
            code_alignment_factor: 1,
            data_alignment_factor: 1,
            return_address_register: Register(3),
            initial_instructions: EndianSlice::new(&instrs1, BigEndian),
        };

        let mut cie2 = CommonInformationEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            version: 4,
            augmentation: None,
            address_size: 4,
            code_alignment_factor: 1,
            data_alignment_factor: 1,
            return_address_register: Register(1),
            initial_instructions: EndianSlice::new(&instrs2, BigEndian),
        };

        let cie1_location = Label::new();
        let cie2_location = Label::new();

        // Write the CIEs first so that their length gets set before we clone
        // them into the FDEs and our equality assertions down the line end up
        // with all the CIEs always having he correct length.
        let kind = debug_frame_be();
        let section = Section::with_endian(kind.endian())
            .mark(&cie1_location)
            .cie(kind, None, &mut cie1)
            .mark(&cie2_location)
            .cie(kind, None, &mut cie2);

        let mut fde1 = FrameDescriptionEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            cie: cie1.clone(),
            initial_address: 0xfeed_beef,
            address_range: 200,
            augmentation: None,
            instructions: EndianSlice::new(&instrs3, BigEndian),
        };

        let mut fde2 = FrameDescriptionEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            cie: cie2.clone(),
            initial_address: 0xfeed_face,
            address_range: 9000,
            augmentation: None,
            instructions: EndianSlice::new(&instrs4, BigEndian),
        };

        let section =
            section
                .fde(kind, &cie1_location, &mut fde1)
                .fde(kind, &cie2_location, &mut fde2);
        section.start().set_const(0);

        let contents = section.get_contents().unwrap();
        let debug_frame = kind.section(&contents);

        // Get the second row of the unwind table in `instrs3`.
        let bases = Default::default();
        let mut ctx = Box::new(UnwindContext::new());
        let result = debug_frame.unwind_info_for_address(
            &bases,
            &mut ctx,
            0xfeed_beef + 150,
            DebugFrame::cie_from_offset,
        );
        assert!(result.is_ok());
        let unwind_info = result.unwrap();

        assert_eq!(
            *unwind_info,
            UnwindTableRow {
                start_address: fde1.initial_address() + 100,
                end_address: fde1.end_address(),
                saved_args_size: 0,
                cfa: CfaRule::RegisterAndOffset {
                    register: Register(4),
                    offset: -12,
                },
                registers: [(Register(0), RegisterRule::Offset(-16))].iter().collect(),
            }
        );
    }

    #[test]
    fn test_unwind_info_for_address_not_found() {
        let debug_frame = DebugFrame::new(&[], NativeEndian);
        let bases = Default::default();
        let mut ctx = Box::new(UnwindContext::new());
        let result = debug_frame.unwind_info_for_address(
            &bases,
            &mut ctx,
            0xbadb_ad99,
            DebugFrame::cie_from_offset,
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::NoUnwindInfoForAddress);
    }

    #[test]
    fn test_eh_frame_hdr_unknown_version() {
        let bases = BaseAddresses::default();
        let buf = &[42];
        let result = EhFrameHdr::new(buf, NativeEndian).parse(&bases, 8);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::UnknownVersion(42));
    }

    #[test]
    fn test_eh_frame_hdr_omit_ehptr() {
        let section = Section::with_endian(Endian::Little)
            .L8(1)
            .L8(0xff)
            .L8(0x03)
            .L8(0x0b)
            .L32(2)
            .L32(10)
            .L32(1)
            .L32(20)
            .L32(2)
            .L32(0);
        let section = section.get_contents().unwrap();
        let bases = BaseAddresses::default();
        let result = EhFrameHdr::new(&section, LittleEndian).parse(&bases, 8);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::CannotParseOmitPointerEncoding);
    }

    #[test]
    fn test_eh_frame_hdr_omit_count() {
        let section = Section::with_endian(Endian::Little)
            .L8(1)
            .L8(0x0b)
            .L8(0xff)
            .L8(0x0b)
            .L32(0x12345);
        let section = section.get_contents().unwrap();
        let bases = BaseAddresses::default();
        let result = EhFrameHdr::new(&section, LittleEndian).parse(&bases, 8);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.eh_frame_ptr(), Pointer::Direct(0x12345));
        assert!(result.table().is_none());
    }

    #[test]
    fn test_eh_frame_hdr_omit_table() {
        let section = Section::with_endian(Endian::Little)
            .L8(1)
            .L8(0x0b)
            .L8(0x03)
            .L8(0xff)
            .L32(0x12345)
            .L32(2);
        let section = section.get_contents().unwrap();
        let bases = BaseAddresses::default();
        let result = EhFrameHdr::new(&section, LittleEndian).parse(&bases, 8);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.eh_frame_ptr(), Pointer::Direct(0x12345));
        assert!(result.table().is_none());
    }

    #[test]
    fn test_eh_frame_hdr_varlen_table() {
        let section = Section::with_endian(Endian::Little)
            .L8(1)
            .L8(0x0b)
            .L8(0x03)
            .L8(0x01)
            .L32(0x12345)
            .L32(2);
        let section = section.get_contents().unwrap();
        let bases = BaseAddresses::default();
        let result = EhFrameHdr::new(&section, LittleEndian).parse(&bases, 8);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.eh_frame_ptr(), Pointer::Direct(0x12345));
        let table = result.table();
        assert!(table.is_some());
        let table = table.unwrap();
        assert_eq!(
            table.lookup(0, &bases),
            Err(Error::VariableLengthSearchTable)
        );
    }

    #[test]
    fn test_eh_frame_hdr_indirect_length() {
        let section = Section::with_endian(Endian::Little)
            .L8(1)
            .L8(0x0b)
            .L8(0x83)
            .L8(0x0b)
            .L32(0x12345)
            .L32(2);
        let section = section.get_contents().unwrap();
        let bases = BaseAddresses::default();
        let result = EhFrameHdr::new(&section, LittleEndian).parse(&bases, 8);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::UnsupportedPointerEncoding);
    }

    #[test]
    fn test_eh_frame_hdr_indirect_ptrs() {
        let section = Section::with_endian(Endian::Little)
            .L8(1)
            .L8(0x8b)
            .L8(0x03)
            .L8(0x8b)
            .L32(0x12345)
            .L32(2)
            .L32(10)
            .L32(1)
            .L32(20)
            .L32(2);
        let section = section.get_contents().unwrap();
        let bases = BaseAddresses::default();
        let result = EhFrameHdr::new(&section, LittleEndian).parse(&bases, 8);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.eh_frame_ptr(), Pointer::Indirect(0x12345));
        let table = result.table();
        assert!(table.is_some());
        let table = table.unwrap();
        assert_eq!(
            table.lookup(0, &bases),
            Err(Error::UnsupportedPointerEncoding)
        );
    }

    #[test]
    fn test_eh_frame_hdr_good() {
        let section = Section::with_endian(Endian::Little)
            .L8(1)
            .L8(0x0b)
            .L8(0x03)
            .L8(0x0b)
            .L32(0x12345)
            .L32(2)
            .L32(10)
            .L32(1)
            .L32(20)
            .L32(2);
        let section = section.get_contents().unwrap();
        let bases = BaseAddresses::default();
        let result = EhFrameHdr::new(&section, LittleEndian).parse(&bases, 8);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.eh_frame_ptr(), Pointer::Direct(0x12345));
        let table = result.table();
        assert!(table.is_some());
        let table = table.unwrap();
        assert_eq!(table.lookup(0, &bases), Ok(Pointer::Direct(1)));
        assert_eq!(table.lookup(9, &bases), Ok(Pointer::Direct(1)));
        assert_eq!(table.lookup(10, &bases), Ok(Pointer::Direct(1)));
        assert_eq!(table.lookup(11, &bases), Ok(Pointer::Direct(1)));
        assert_eq!(table.lookup(19, &bases), Ok(Pointer::Direct(1)));
        assert_eq!(table.lookup(20, &bases), Ok(Pointer::Direct(2)));
        assert_eq!(table.lookup(21, &bases), Ok(Pointer::Direct(2)));
        assert_eq!(table.lookup(100_000, &bases), Ok(Pointer::Direct(2)));
    }

    #[test]
    fn test_eh_frame_fde_for_address_good() {
        // First, setup eh_frame
        // Write the CIE first so that its length gets set before we clone it
        // into the FDE.
        let mut cie = make_test_cie();
        cie.format = Format::Dwarf32;
        cie.version = 1;

        let start_of_cie = Label::new();
        let end_of_cie = Label::new();

        let kind = eh_frame_le();
        let section = Section::with_endian(kind.endian())
            .append_repeated(0, 16)
            .mark(&start_of_cie)
            .cie(kind, None, &mut cie)
            .mark(&end_of_cie);

        let mut fde1 = FrameDescriptionEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            cie: cie.clone(),
            initial_address: 9,
            address_range: 4,
            augmentation: None,
            instructions: EndianSlice::new(&[], LittleEndian),
        };
        let mut fde2 = FrameDescriptionEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            cie: cie.clone(),
            initial_address: 20,
            address_range: 8,
            augmentation: None,
            instructions: EndianSlice::new(&[], LittleEndian),
        };

        let start_of_fde1 = Label::new();
        let start_of_fde2 = Label::new();

        let section = section
            // +4 for the FDE length before the CIE offset.
            .mark(&start_of_fde1)
            .fde(kind, (&start_of_fde1 - &start_of_cie + 4) as u64, &mut fde1)
            .mark(&start_of_fde2)
            .fde(kind, (&start_of_fde2 - &start_of_cie + 4) as u64, &mut fde2);

        section.start().set_const(0);
        let section = section.get_contents().unwrap();
        let eh_frame = kind.section(&section);

        // Setup eh_frame_hdr
        let section = Section::with_endian(kind.endian())
            .L8(1)
            .L8(0x0b)
            .L8(0x03)
            .L8(0x0b)
            .L32(0x12345)
            .L32(2)
            .L32(10)
            .L32(0x12345 + start_of_fde1.value().unwrap() as u32)
            .L32(20)
            .L32(0x12345 + start_of_fde2.value().unwrap() as u32);

        let section = section.get_contents().unwrap();
        let bases = BaseAddresses::default();
        let eh_frame_hdr = EhFrameHdr::new(&section, LittleEndian).parse(&bases, 8);
        assert!(eh_frame_hdr.is_ok());
        let eh_frame_hdr = eh_frame_hdr.unwrap();

        let table = eh_frame_hdr.table();
        assert!(table.is_some());
        let table = table.unwrap();

        let bases = Default::default();
        let mut iter = table.iter(&bases);
        assert_eq!(
            iter.next(),
            Ok(Some((
                Pointer::Direct(10),
                Pointer::Direct(0x12345 + start_of_fde1.value().unwrap())
            )))
        );
        assert_eq!(
            iter.next(),
            Ok(Some((
                Pointer::Direct(20),
                Pointer::Direct(0x12345 + start_of_fde2.value().unwrap())
            )))
        );
        assert_eq!(iter.next(), Ok(None));

        assert_eq!(
            table.iter(&bases).nth(0),
            Ok(Some((
                Pointer::Direct(10),
                Pointer::Direct(0x12345 + start_of_fde1.value().unwrap())
            )))
        );

        assert_eq!(
            table.iter(&bases).nth(1),
            Ok(Some((
                Pointer::Direct(20),
                Pointer::Direct(0x12345 + start_of_fde2.value().unwrap())
            )))
        );
        assert_eq!(table.iter(&bases).nth(2), Ok(None));

        let f = |_: &_, _: &_, o: EhFrameOffset| {
            assert_eq!(o, EhFrameOffset(start_of_cie.value().unwrap() as usize));
            Ok(cie.clone())
        };
        assert_eq!(
            table.fde_for_address(&eh_frame, &bases, 9, f),
            Ok(fde1.clone())
        );
        assert_eq!(
            table.fde_for_address(&eh_frame, &bases, 10, f),
            Ok(fde1.clone())
        );
        assert_eq!(table.fde_for_address(&eh_frame, &bases, 11, f), Ok(fde1));
        assert_eq!(
            table.fde_for_address(&eh_frame, &bases, 19, f),
            Err(Error::NoUnwindInfoForAddress)
        );
        assert_eq!(
            table.fde_for_address(&eh_frame, &bases, 20, f),
            Ok(fde2.clone())
        );
        assert_eq!(table.fde_for_address(&eh_frame, &bases, 21, f), Ok(fde2));
        assert_eq!(
            table.fde_for_address(&eh_frame, &bases, 100_000, f),
            Err(Error::NoUnwindInfoForAddress)
        );
    }

    #[test]
    fn test_eh_frame_stops_at_zero_length() {
        let mut cie = make_test_cie();
        let kind = eh_frame_le();
        let section = Section::with_endian(Endian::Little)
            .L32(0)
            .cie(kind, None, &mut cie)
            .L32(0);
        let contents = section.get_contents().unwrap();
        let eh_frame = kind.section(&contents);
        let bases = Default::default();

        let mut entries = eh_frame.entries(&bases);
        assert_eq!(entries.next(), Ok(None));

        assert_eq!(
            eh_frame.cie_from_offset(&bases, EhFrameOffset(0)),
            Err(Error::NoEntryAtGivenOffset)
        );
    }

    #[test]
    fn test_debug_frame_skips_zero_length() {
        let mut cie = make_test_cie();
        let kind = debug_frame_le();
        let section = Section::with_endian(Endian::Little)
            .L32(0)
            .cie(kind, None, &mut cie)
            .L32(0);
        let contents = section.get_contents().unwrap();
        let debug_frame = kind.section(&contents);
        let bases = Default::default();

        let mut entries = debug_frame.entries(&bases);
        assert_eq!(entries.next(), Ok(Some(CieOrFde::Cie(cie))));
        assert_eq!(entries.next(), Ok(None));

        assert_eq!(
            debug_frame.cie_from_offset(&bases, DebugFrameOffset(0)),
            Err(Error::NoEntryAtGivenOffset)
        );
    }

    fn resolve_cie_offset(buf: &[u8], cie_offset: usize) -> Result<usize> {
        let mut fde = FrameDescriptionEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf64,
            cie: make_test_cie(),
            initial_address: 0xfeed_beef,
            address_range: 39,
            augmentation: None,
            instructions: EndianSlice::new(&[], LittleEndian),
        };

        let kind = eh_frame_le();
        let section = Section::with_endian(kind.endian())
            .append_bytes(buf)
            .fde(kind, cie_offset as u64, &mut fde)
            .append_bytes(buf);

        let section = section.get_contents().unwrap();
        let eh_frame = kind.section(&section);
        let input = &mut EndianSlice::new(&section[buf.len()..], LittleEndian);

        let bases = Default::default();
        match parse_cfi_entry(&bases, &eh_frame, input) {
            Ok(Some(CieOrFde::Fde(partial))) => Ok(partial.cie_offset.0),
            Err(e) => Err(e),
            otherwise => panic!("Unexpected result: {:#?}", otherwise),
        }
    }

    #[test]
    fn test_eh_frame_resolve_cie_offset_ok() {
        let buf = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let cie_offset = 2;
        // + 4 for size of length field
        assert_eq!(
            resolve_cie_offset(&buf, buf.len() + 4 - cie_offset),
            Ok(cie_offset)
        );
    }

    #[test]
    fn test_eh_frame_resolve_cie_offset_out_of_bounds() {
        let buf = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        assert_eq!(
            resolve_cie_offset(&buf, buf.len() + 4 + 2),
            Err(Error::OffsetOutOfBounds)
        );
    }

    #[test]
    fn test_eh_frame_resolve_cie_offset_underflow() {
        let buf = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        assert_eq!(
            resolve_cie_offset(&buf, usize::MAX),
            Err(Error::OffsetOutOfBounds)
        );
    }

    #[test]
    fn test_eh_frame_fde_ok() {
        let mut cie = make_test_cie();
        cie.format = Format::Dwarf32;
        cie.version = 1;

        let start_of_cie = Label::new();
        let end_of_cie = Label::new();

        // Write the CIE first so that its length gets set before we clone it
        // into the FDE.
        let kind = eh_frame_le();
        let section = Section::with_endian(kind.endian())
            .append_repeated(0, 16)
            .mark(&start_of_cie)
            .cie(kind, None, &mut cie)
            .mark(&end_of_cie);

        let mut fde = FrameDescriptionEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            cie: cie.clone(),
            initial_address: 0xfeed_beef,
            address_range: 999,
            augmentation: None,
            instructions: EndianSlice::new(&[], LittleEndian),
        };

        let section = section
            // +4 for the FDE length before the CIE offset.
            .fde(kind, (&end_of_cie - &start_of_cie + 4) as u64, &mut fde);

        section.start().set_const(0);
        let section = section.get_contents().unwrap();
        let eh_frame = kind.section(&section);
        let section = EndianSlice::new(&section, LittleEndian);

        let mut offset = None;
        let result = parse_fde(
            eh_frame,
            &mut section.range_from(end_of_cie.value().unwrap() as usize..),
            |_, _, o| {
                offset = Some(o);
                assert_eq!(o, EhFrameOffset(start_of_cie.value().unwrap() as usize));
                Ok(cie.clone())
            },
        );
        match result {
            Ok(actual) => assert_eq!(actual, fde),
            otherwise => panic!("Unexpected result {:?}", otherwise),
        }
        assert!(offset.is_some());
    }

    #[test]
    fn test_eh_frame_fde_out_of_bounds() {
        let mut cie = make_test_cie();
        cie.version = 1;

        let end_of_cie = Label::new();

        let mut fde = FrameDescriptionEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf64,
            cie: cie.clone(),
            initial_address: 0xfeed_beef,
            address_range: 999,
            augmentation: None,
            instructions: EndianSlice::new(&[], LittleEndian),
        };

        let kind = eh_frame_le();
        let section = Section::with_endian(kind.endian())
            .cie(kind, None, &mut cie)
            .mark(&end_of_cie)
            .fde(kind, 99_999_999_999_999, &mut fde);

        section.start().set_const(0);
        let section = section.get_contents().unwrap();
        let eh_frame = kind.section(&section);
        let section = EndianSlice::new(&section, LittleEndian);

        let result = parse_fde(
            eh_frame,
            &mut section.range_from(end_of_cie.value().unwrap() as usize..),
            UnwindSection::cie_from_offset,
        );
        assert_eq!(result, Err(Error::OffsetOutOfBounds));
    }

    #[test]
    fn test_augmentation_parse_not_z_augmentation() {
        let augmentation = &mut EndianSlice::new(b"wtf", NativeEndian);
        let bases = Default::default();
        let address_size = 8;
        let section = EhFrame::new(&[], NativeEndian);
        let input = &mut EndianSlice::new(&[], NativeEndian);
        assert_eq!(
            Augmentation::parse(augmentation, &bases, address_size, &section, input),
            Err(Error::UnknownAugmentation)
        );
    }

    #[test]
    fn test_augmentation_parse_just_signal_trampoline() {
        let aug_str = &mut EndianSlice::new(b"S", LittleEndian);
        let bases = Default::default();
        let address_size = 8;
        let section = EhFrame::new(&[], LittleEndian);
        let input = &mut EndianSlice::new(&[], LittleEndian);

        let augmentation = Augmentation {
            is_signal_trampoline: true,
            ..Default::default()
        };

        assert_eq!(
            Augmentation::parse(aug_str, &bases, address_size, &section, input),
            Ok(augmentation)
        );
    }

    #[test]
    fn test_augmentation_parse_unknown_part_of_z_augmentation() {
        // The 'Z' character is not defined by the z-style augmentation.
        let bases = Default::default();
        let address_size = 8;
        let section = Section::with_endian(Endian::Little)
            .uleb(4)
            .append_repeated(4, 4)
            .get_contents()
            .unwrap();
        let section = EhFrame::new(&section, LittleEndian);
        let input = &mut section.section().clone();
        let augmentation = &mut EndianSlice::new(b"zZ", LittleEndian);
        assert_eq!(
            Augmentation::parse(augmentation, &bases, address_size, &section, input),
            Err(Error::UnknownAugmentation)
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_augmentation_parse_L() {
        let bases = Default::default();
        let address_size = 8;
        let rest = [9, 8, 7, 6, 5, 4, 3, 2, 1];

        let section = Section::with_endian(Endian::Little)
            .uleb(1)
            .D8(constants::DW_EH_PE_uleb128.0)
            .append_bytes(&rest)
            .get_contents()
            .unwrap();
        let section = EhFrame::new(&section, LittleEndian);
        let input = &mut section.section().clone();
        let aug_str = &mut EndianSlice::new(b"zL", LittleEndian);

        let augmentation = Augmentation {
            lsda: Some(constants::DW_EH_PE_uleb128),
            ..Default::default()
        };

        assert_eq!(
            Augmentation::parse(aug_str, &bases, address_size, &section, input),
            Ok(augmentation)
        );
        assert_eq!(*input, EndianSlice::new(&rest, LittleEndian));
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_augmentation_parse_P() {
        let bases = Default::default();
        let address_size = 8;
        let rest = [9, 8, 7, 6, 5, 4, 3, 2, 1];

        let section = Section::with_endian(Endian::Little)
            .uleb(9)
            .D8(constants::DW_EH_PE_udata8.0)
            .L64(0xf00d_f00d)
            .append_bytes(&rest)
            .get_contents()
            .unwrap();
        let section = EhFrame::new(&section, LittleEndian);
        let input = &mut section.section().clone();
        let aug_str = &mut EndianSlice::new(b"zP", LittleEndian);

        let augmentation = Augmentation {
            personality: Some((constants::DW_EH_PE_udata8, Pointer::Direct(0xf00d_f00d))),
            ..Default::default()
        };

        assert_eq!(
            Augmentation::parse(aug_str, &bases, address_size, &section, input),
            Ok(augmentation)
        );
        assert_eq!(*input, EndianSlice::new(&rest, LittleEndian));
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_augmentation_parse_R() {
        let bases = Default::default();
        let address_size = 8;
        let rest = [9, 8, 7, 6, 5, 4, 3, 2, 1];

        let section = Section::with_endian(Endian::Little)
            .uleb(1)
            .D8(constants::DW_EH_PE_udata4.0)
            .append_bytes(&rest)
            .get_contents()
            .unwrap();
        let section = EhFrame::new(&section, LittleEndian);
        let input = &mut section.section().clone();
        let aug_str = &mut EndianSlice::new(b"zR", LittleEndian);

        let augmentation = Augmentation {
            fde_address_encoding: Some(constants::DW_EH_PE_udata4),
            ..Default::default()
        };

        assert_eq!(
            Augmentation::parse(aug_str, &bases, address_size, &section, input),
            Ok(augmentation)
        );
        assert_eq!(*input, EndianSlice::new(&rest, LittleEndian));
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_augmentation_parse_S() {
        let bases = Default::default();
        let address_size = 8;
        let rest = [9, 8, 7, 6, 5, 4, 3, 2, 1];

        let section = Section::with_endian(Endian::Little)
            .uleb(0)
            .append_bytes(&rest)
            .get_contents()
            .unwrap();
        let section = EhFrame::new(&section, LittleEndian);
        let input = &mut section.section().clone();
        let aug_str = &mut EndianSlice::new(b"zS", LittleEndian);

        let augmentation = Augmentation {
            is_signal_trampoline: true,
            ..Default::default()
        };

        assert_eq!(
            Augmentation::parse(aug_str, &bases, address_size, &section, input),
            Ok(augmentation)
        );
        assert_eq!(*input, EndianSlice::new(&rest, LittleEndian));
    }

    #[test]
    fn test_augmentation_parse_all() {
        let bases = Default::default();
        let address_size = 8;
        let rest = [9, 8, 7, 6, 5, 4, 3, 2, 1];

        let section = Section::with_endian(Endian::Little)
            .uleb(1 + 9 + 1)
            // L
            .D8(constants::DW_EH_PE_uleb128.0)
            // P
            .D8(constants::DW_EH_PE_udata8.0)
            .L64(0x1bad_f00d)
            // R
            .D8(constants::DW_EH_PE_uleb128.0)
            .append_bytes(&rest)
            .get_contents()
            .unwrap();
        let section = EhFrame::new(&section, LittleEndian);
        let input = &mut section.section().clone();
        let aug_str = &mut EndianSlice::new(b"zLPRS", LittleEndian);

        let augmentation = Augmentation {
            lsda: Some(constants::DW_EH_PE_uleb128),
            personality: Some((constants::DW_EH_PE_udata8, Pointer::Direct(0x1bad_f00d))),
            fde_address_encoding: Some(constants::DW_EH_PE_uleb128),
            is_signal_trampoline: true,
        };

        assert_eq!(
            Augmentation::parse(aug_str, &bases, address_size, &section, input),
            Ok(augmentation)
        );
        assert_eq!(*input, EndianSlice::new(&rest, LittleEndian));
    }

    #[test]
    fn test_eh_frame_fde_no_augmentation() {
        let instrs = [1, 2, 3, 4];
        let cie_offset = 1;

        let mut cie = make_test_cie();
        cie.format = Format::Dwarf32;
        cie.version = 1;

        let mut fde = FrameDescriptionEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            cie: cie.clone(),
            initial_address: 0xfeed_face,
            address_range: 9000,
            augmentation: None,
            instructions: EndianSlice::new(&instrs, LittleEndian),
        };

        let rest = [1, 2, 3, 4];

        let kind = eh_frame_le();
        let section = Section::with_endian(kind.endian())
            .fde(kind, cie_offset, &mut fde)
            .append_bytes(&rest)
            .get_contents()
            .unwrap();
        let section = kind.section(&section);
        let input = &mut section.section().clone();

        let result = parse_fde(section, input, |_, _, _| Ok(cie.clone()));
        assert_eq!(result, Ok(fde));
        assert_eq!(*input, EndianSlice::new(&rest, LittleEndian));
    }

    #[test]
    fn test_eh_frame_fde_empty_augmentation() {
        let instrs = [1, 2, 3, 4];
        let cie_offset = 1;

        let mut cie = make_test_cie();
        cie.format = Format::Dwarf32;
        cie.version = 1;
        cie.augmentation = Some(Augmentation::default());

        let mut fde = FrameDescriptionEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            cie: cie.clone(),
            initial_address: 0xfeed_face,
            address_range: 9000,
            augmentation: Some(AugmentationData::default()),
            instructions: EndianSlice::new(&instrs, LittleEndian),
        };

        let rest = [1, 2, 3, 4];

        let kind = eh_frame_le();
        let section = Section::with_endian(kind.endian())
            .fde(kind, cie_offset, &mut fde)
            .append_bytes(&rest)
            .get_contents()
            .unwrap();
        let section = kind.section(&section);
        let input = &mut section.section().clone();

        let result = parse_fde(section, input, |_, _, _| Ok(cie.clone()));
        assert_eq!(result, Ok(fde));
        assert_eq!(*input, EndianSlice::new(&rest, LittleEndian));
    }

    #[test]
    fn test_eh_frame_fde_lsda_augmentation() {
        let instrs = [1, 2, 3, 4];
        let cie_offset = 1;

        let mut cie = make_test_cie();
        cie.format = Format::Dwarf32;
        cie.version = 1;
        cie.augmentation = Some(Augmentation::default());
        cie.augmentation.as_mut().unwrap().lsda = Some(constants::DW_EH_PE_absptr);

        let mut fde = FrameDescriptionEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            cie: cie.clone(),
            initial_address: 0xfeed_face,
            address_range: 9000,
            augmentation: Some(AugmentationData {
                lsda: Some(Pointer::Direct(0x1122_3344)),
            }),
            instructions: EndianSlice::new(&instrs, LittleEndian),
        };

        let rest = [1, 2, 3, 4];

        let kind = eh_frame_le();
        let section = Section::with_endian(kind.endian())
            .fde(kind, cie_offset, &mut fde)
            .append_bytes(&rest)
            .get_contents()
            .unwrap();
        let section = kind.section(&section);
        let input = &mut section.section().clone();

        let result = parse_fde(section, input, |_, _, _| Ok(cie.clone()));
        assert_eq!(result, Ok(fde));
        assert_eq!(*input, EndianSlice::new(&rest, LittleEndian));
    }

    #[test]
    fn test_eh_frame_fde_lsda_function_relative() {
        let instrs = [1, 2, 3, 4];
        let cie_offset = 1;

        let mut cie = make_test_cie();
        cie.format = Format::Dwarf32;
        cie.version = 1;
        cie.augmentation = Some(Augmentation::default());
        cie.augmentation.as_mut().unwrap().lsda =
            Some(constants::DW_EH_PE_funcrel | constants::DW_EH_PE_absptr);

        let mut fde = FrameDescriptionEntry {
            offset: 0,
            length: 0,
            format: Format::Dwarf32,
            cie: cie.clone(),
            initial_address: 0xfeed_face,
            address_range: 9000,
            augmentation: Some(AugmentationData {
                lsda: Some(Pointer::Direct(0xbeef)),
            }),
            instructions: EndianSlice::new(&instrs, LittleEndian),
        };

        let rest = [1, 2, 3, 4];

        let kind = eh_frame_le();
        let section = Section::with_endian(kind.endian())
            .append_repeated(10, 10)
            .fde(kind, cie_offset, &mut fde)
            .append_bytes(&rest)
            .get_contents()
            .unwrap();
        let section = kind.section(&section);
        let input = &mut section.section().range_from(10..);

        // Adjust the FDE's augmentation to be relative to the function.
        fde.augmentation.as_mut().unwrap().lsda = Some(Pointer::Direct(0xfeed_face + 0xbeef));

        let result = parse_fde(section, input, |_, _, _| Ok(cie.clone()));
        assert_eq!(result, Ok(fde));
        assert_eq!(*input, EndianSlice::new(&rest, LittleEndian));
    }

    #[test]
    fn test_eh_frame_cie_personality_function_relative_bad_context() {
        let instrs = [1, 2, 3, 4];

        let length = Label::new();
        let start = Label::new();
        let end = Label::new();

        let aug_len = Label::new();
        let aug_start = Label::new();
        let aug_end = Label::new();

        let section = Section::with_endian(Endian::Little)
            // Length
            .L32(&length)
            .mark(&start)
            // CIE ID
            .L32(0)
            // Version
            .D8(1)
            // Augmentation
            .append_bytes(b"zP\0")
            // Code alignment factor
            .uleb(1)
            // Data alignment factor
            .sleb(1)
            // Return address register
            .uleb(1)
            // Augmentation data length. This is a uleb, be we rely on the value
            // being less than 2^7 and therefore a valid uleb (can't use Label
            // with uleb).
            .D8(&aug_len)
            .mark(&aug_start)
            // Augmentation data. Personality encoding and then encoded pointer.
            .D8(constants::DW_EH_PE_funcrel.0 | constants::DW_EH_PE_uleb128.0)
            .uleb(1)
            .mark(&aug_end)
            // Initial instructions
            .append_bytes(&instrs)
            .mark(&end);

        length.set_const((&end - &start) as u64);
        aug_len.set_const((&aug_end - &aug_start) as u64);

        let section = section.get_contents().unwrap();
        let section = EhFrame::new(&section, LittleEndian);

        let bases = BaseAddresses::default();
        let mut iter = section.entries(&bases);
        assert_eq!(iter.next(), Err(Error::FuncRelativePointerInBadContext));
    }

    #[test]
    fn register_rule_map_eq() {
        // Different order, but still equal.
        let map1: RegisterRuleMap<usize> = [
            (Register(0), RegisterRule::SameValue),
            (Register(3), RegisterRule::Offset(1)),
        ]
        .iter()
        .collect();
        let map2: RegisterRuleMap<usize> = [
            (Register(3), RegisterRule::Offset(1)),
            (Register(0), RegisterRule::SameValue),
        ]
        .iter()
        .collect();
        assert_eq!(map1, map2);
        assert_eq!(map2, map1);

        // Not equal.
        let map3: RegisterRuleMap<usize> = [
            (Register(0), RegisterRule::SameValue),
            (Register(2), RegisterRule::Offset(1)),
        ]
        .iter()
        .collect();
        let map4: RegisterRuleMap<usize> = [
            (Register(3), RegisterRule::Offset(1)),
            (Register(0), RegisterRule::SameValue),
        ]
        .iter()
        .collect();
        assert!(map3 != map4);
        assert!(map4 != map3);

        // One has undefined explicitly set, other implicitly has undefined.
        let mut map5 = RegisterRuleMap::<usize>::default();
        map5.set(Register(0), RegisterRule::SameValue).unwrap();
        map5.set(Register(0), RegisterRule::Undefined).unwrap();
        let map6 = RegisterRuleMap::<usize>::default();
        assert_eq!(map5, map6);
        assert_eq!(map6, map5);
    }

    #[test]
    fn iter_register_rules() {
        let row = UnwindTableRow::<usize> {
            registers: [
                (Register(0), RegisterRule::SameValue),
                (Register(1), RegisterRule::Offset(1)),
                (Register(2), RegisterRule::ValOffset(2)),
            ]
            .iter()
            .collect(),
            ..Default::default()
        };

        let mut found0 = false;
        let mut found1 = false;
        let mut found2 = false;

        for &(register, ref rule) in row.registers() {
            match register.0 {
                0 => {
                    assert!(!found0);
                    found0 = true;
                    assert_eq!(*rule, RegisterRule::SameValue);
                }
                1 => {
                    assert!(!found1);
                    found1 = true;
                    assert_eq!(*rule, RegisterRule::Offset(1));
                }
                2 => {
                    assert!(!found2);
                    found2 = true;
                    assert_eq!(*rule, RegisterRule::ValOffset(2));
                }
                x => panic!("Unexpected register rule: ({}, {:?})", x, rule),
            }
        }

        assert!(found0);
        assert!(found1);
        assert!(found2);
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn size_of_unwind_ctx() {
        use core::mem;
        let size = mem::size_of::<UnwindContext<usize>>();
        let max_size = 30968;
        if size > max_size {
            assert_eq!(size, max_size);
        }
    }

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn size_of_register_rule_map() {
        use core::mem;
        let size = mem::size_of::<RegisterRuleMap<usize>>();
        let max_size = 6152;
        if size > max_size {
            assert_eq!(size, max_size);
        }
    }

    #[test]
    fn test_parse_pointer_encoding_ok() {
        use crate::endianity::NativeEndian;
        let expected = constants::DW_EH_PE_uleb128 | constants::DW_EH_PE_pcrel;
        let input = [expected.0, 1, 2, 3, 4];
        let input = &mut EndianSlice::new(&input, NativeEndian);
        assert_eq!(parse_pointer_encoding(input), Ok(expected));
        assert_eq!(*input, EndianSlice::new(&[1, 2, 3, 4], NativeEndian));
    }

    #[test]
    fn test_parse_pointer_encoding_bad_encoding() {
        use crate::endianity::NativeEndian;
        let expected =
            constants::DwEhPe((constants::DW_EH_PE_sdata8.0 + 1) | constants::DW_EH_PE_pcrel.0);
        let input = [expected.0, 1, 2, 3, 4];
        let input = &mut EndianSlice::new(&input, NativeEndian);
        assert_eq!(
            Err(Error::UnknownPointerEncoding(expected)),
            parse_pointer_encoding(input)
        );
    }

    #[test]
    fn test_parse_encoded_pointer_absptr() {
        let encoding = constants::DW_EH_PE_absptr;
        let expected_rest = [1, 2, 3, 4];

        let input = Section::with_endian(Endian::Little)
            .L32(0xf00d_f00d)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        let parameters = PointerEncodingParameters {
            bases: &SectionBaseAddresses::default(),
            func_base: None,
            address_size: 4,
            section: &input,
        };
        assert_eq!(
            parse_encoded_pointer(encoding, &parameters, &mut rest),
            Ok(Pointer::Direct(0xf00d_f00d))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_encoded_pointer_pcrel() {
        let encoding = constants::DW_EH_PE_pcrel;
        let expected_rest = [1, 2, 3, 4];

        let input = Section::with_endian(Endian::Little)
            .append_repeated(0, 0x10)
            .L32(0x1)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input.range_from(0x10..);

        let parameters = PointerEncodingParameters {
            bases: &BaseAddresses::default().set_eh_frame(0x100).eh_frame,
            func_base: None,
            address_size: 4,
            section: &input,
        };
        assert_eq!(
            parse_encoded_pointer(encoding, &parameters, &mut rest),
            Ok(Pointer::Direct(0x111))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_encoded_pointer_pcrel_undefined() {
        let encoding = constants::DW_EH_PE_pcrel;

        let input = Section::with_endian(Endian::Little).L32(0x1);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        let parameters = PointerEncodingParameters {
            bases: &SectionBaseAddresses::default(),
            func_base: None,
            address_size: 4,
            section: &input,
        };
        assert_eq!(
            parse_encoded_pointer(encoding, &parameters, &mut rest),
            Err(Error::PcRelativePointerButSectionBaseIsUndefined)
        );
    }

    #[test]
    fn test_parse_encoded_pointer_textrel() {
        let encoding = constants::DW_EH_PE_textrel;
        let expected_rest = [1, 2, 3, 4];

        let input = Section::with_endian(Endian::Little)
            .L32(0x1)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        let parameters = PointerEncodingParameters {
            bases: &BaseAddresses::default().set_text(0x10).eh_frame,
            func_base: None,
            address_size: 4,
            section: &input,
        };
        assert_eq!(
            parse_encoded_pointer(encoding, &parameters, &mut rest),
            Ok(Pointer::Direct(0x11))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_encoded_pointer_textrel_undefined() {
        let encoding = constants::DW_EH_PE_textrel;

        let input = Section::with_endian(Endian::Little).L32(0x1);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        let parameters = PointerEncodingParameters {
            bases: &SectionBaseAddresses::default(),
            func_base: None,
            address_size: 4,
            section: &input,
        };
        assert_eq!(
            parse_encoded_pointer(encoding, &parameters, &mut rest),
            Err(Error::TextRelativePointerButTextBaseIsUndefined)
        );
    }

    #[test]
    fn test_parse_encoded_pointer_datarel() {
        let encoding = constants::DW_EH_PE_datarel;
        let expected_rest = [1, 2, 3, 4];

        let input = Section::with_endian(Endian::Little)
            .L32(0x1)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        let parameters = PointerEncodingParameters {
            bases: &BaseAddresses::default().set_got(0x10).eh_frame,
            func_base: None,
            address_size: 4,
            section: &input,
        };
        assert_eq!(
            parse_encoded_pointer(encoding, &parameters, &mut rest),
            Ok(Pointer::Direct(0x11))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_encoded_pointer_datarel_undefined() {
        let encoding = constants::DW_EH_PE_datarel;

        let input = Section::with_endian(Endian::Little).L32(0x1);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        let parameters = PointerEncodingParameters {
            bases: &SectionBaseAddresses::default(),
            func_base: None,
            address_size: 4,
            section: &input,
        };
        assert_eq!(
            parse_encoded_pointer(encoding, &parameters, &mut rest),
            Err(Error::DataRelativePointerButDataBaseIsUndefined)
        );
    }

    #[test]
    fn test_parse_encoded_pointer_funcrel() {
        let encoding = constants::DW_EH_PE_funcrel;
        let expected_rest = [1, 2, 3, 4];

        let input = Section::with_endian(Endian::Little)
            .L32(0x1)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        let parameters = PointerEncodingParameters {
            bases: &SectionBaseAddresses::default(),
            func_base: Some(0x10),
            address_size: 4,
            section: &input,
        };
        assert_eq!(
            parse_encoded_pointer(encoding, &parameters, &mut rest),
            Ok(Pointer::Direct(0x11))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_encoded_pointer_funcrel_undefined() {
        let encoding = constants::DW_EH_PE_funcrel;

        let input = Section::with_endian(Endian::Little).L32(0x1);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        let parameters = PointerEncodingParameters {
            bases: &SectionBaseAddresses::default(),
            func_base: None,
            address_size: 4,
            section: &input,
        };
        assert_eq!(
            parse_encoded_pointer(encoding, &parameters, &mut rest),
            Err(Error::FuncRelativePointerInBadContext)
        );
    }

    #[test]
    fn test_parse_encoded_pointer_uleb128() {
        let encoding = constants::DW_EH_PE_absptr | constants::DW_EH_PE_uleb128;
        let expected_rest = [1, 2, 3, 4];

        let input = Section::with_endian(Endian::Little)
            .uleb(0x12_3456)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        let parameters = PointerEncodingParameters {
            bases: &SectionBaseAddresses::default(),
            func_base: None,
            address_size: 4,
            section: &input,
        };
        assert_eq!(
            parse_encoded_pointer(encoding, &parameters, &mut rest),
            Ok(Pointer::Direct(0x12_3456))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_encoded_pointer_udata2() {
        let encoding = constants::DW_EH_PE_absptr | constants::DW_EH_PE_udata2;
        let expected_rest = [1, 2, 3, 4];

        let input = Section::with_endian(Endian::Little)
            .L16(0x1234)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        let parameters = PointerEncodingParameters {
            bases: &SectionBaseAddresses::default(),
            func_base: None,
            address_size: 4,
            section: &input,
        };
        assert_eq!(
            parse_encoded_pointer(encoding, &parameters, &mut rest),
            Ok(Pointer::Direct(0x1234))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_encoded_pointer_udata4() {
        let encoding = constants::DW_EH_PE_absptr | constants::DW_EH_PE_udata4;
        let expected_rest = [1, 2, 3, 4];

        let input = Section::with_endian(Endian::Little)
            .L32(0x1234_5678)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        let parameters = PointerEncodingParameters {
            bases: &SectionBaseAddresses::default(),
            func_base: None,
            address_size: 4,
            section: &input,
        };
        assert_eq!(
            parse_encoded_pointer(encoding, &parameters, &mut rest),
            Ok(Pointer::Direct(0x1234_5678))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_encoded_pointer_udata8() {
        let encoding = constants::DW_EH_PE_absptr | constants::DW_EH_PE_udata8;
        let expected_rest = [1, 2, 3, 4];

        let input = Section::with_endian(Endian::Little)
            .L64(0x1234_5678_1234_5678)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        let parameters = PointerEncodingParameters {
            bases: &SectionBaseAddresses::default(),
            func_base: None,
            address_size: 8,
            section: &input,
        };
        assert_eq!(
            parse_encoded_pointer(encoding, &parameters, &mut rest),
            Ok(Pointer::Direct(0x1234_5678_1234_5678))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_encoded_pointer_sleb128() {
        let encoding = constants::DW_EH_PE_textrel | constants::DW_EH_PE_sleb128;
        let expected_rest = [1, 2, 3, 4];

        let input = Section::with_endian(Endian::Little)
            .sleb(-0x1111)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        let parameters = PointerEncodingParameters {
            bases: &BaseAddresses::default().set_text(0x1111_1111).eh_frame,
            func_base: None,
            address_size: 4,
            section: &input,
        };
        assert_eq!(
            parse_encoded_pointer(encoding, &parameters, &mut rest),
            Ok(Pointer::Direct(0x1111_0000))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_encoded_pointer_sdata2() {
        let encoding = constants::DW_EH_PE_absptr | constants::DW_EH_PE_sdata2;
        let expected_rest = [1, 2, 3, 4];
        let expected = 0x111_i16;

        let input = Section::with_endian(Endian::Little)
            .L16(expected as u16)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        let parameters = PointerEncodingParameters {
            bases: &SectionBaseAddresses::default(),
            func_base: None,
            address_size: 4,
            section: &input,
        };
        assert_eq!(
            parse_encoded_pointer(encoding, &parameters, &mut rest),
            Ok(Pointer::Direct(expected as u64))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_encoded_pointer_sdata4() {
        let encoding = constants::DW_EH_PE_absptr | constants::DW_EH_PE_sdata4;
        let expected_rest = [1, 2, 3, 4];
        let expected = 0x111_1111_i32;

        let input = Section::with_endian(Endian::Little)
            .L32(expected as u32)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        let parameters = PointerEncodingParameters {
            bases: &SectionBaseAddresses::default(),
            func_base: None,
            address_size: 4,
            section: &input,
        };
        assert_eq!(
            parse_encoded_pointer(encoding, &parameters, &mut rest),
            Ok(Pointer::Direct(expected as u64))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_encoded_pointer_sdata8() {
        let encoding = constants::DW_EH_PE_absptr | constants::DW_EH_PE_sdata8;
        let expected_rest = [1, 2, 3, 4];
        let expected = -0x11_1111_1222_2222_i64;

        let input = Section::with_endian(Endian::Little)
            .L64(expected as u64)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        let parameters = PointerEncodingParameters {
            bases: &SectionBaseAddresses::default(),
            func_base: None,
            address_size: 8,
            section: &input,
        };
        assert_eq!(
            parse_encoded_pointer(encoding, &parameters, &mut rest),
            Ok(Pointer::Direct(expected as u64))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_encoded_pointer_omit() {
        let encoding = constants::DW_EH_PE_omit;

        let input = Section::with_endian(Endian::Little).L32(0x1);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        let parameters = PointerEncodingParameters {
            bases: &SectionBaseAddresses::default(),
            func_base: None,
            address_size: 4,
            section: &input,
        };
        assert_eq!(
            parse_encoded_pointer(encoding, &parameters, &mut rest),
            Err(Error::CannotParseOmitPointerEncoding)
        );
        assert_eq!(rest, input);
    }

    #[test]
    fn test_parse_encoded_pointer_bad_encoding() {
        let encoding = constants::DwEhPe(constants::DW_EH_PE_sdata8.0 + 1);

        let input = Section::with_endian(Endian::Little).L32(0x1);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        let parameters = PointerEncodingParameters {
            bases: &SectionBaseAddresses::default(),
            func_base: None,
            address_size: 4,
            section: &input,
        };
        assert_eq!(
            parse_encoded_pointer(encoding, &parameters, &mut rest),
            Err(Error::UnknownPointerEncoding(encoding))
        );
    }

    #[test]
    fn test_parse_encoded_pointer_aligned() {
        // FIXME: support this encoding!

        let encoding = constants::DW_EH_PE_aligned;

        let input = Section::with_endian(Endian::Little).L32(0x1);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        let parameters = PointerEncodingParameters {
            bases: &SectionBaseAddresses::default(),
            func_base: None,
            address_size: 4,
            section: &input,
        };
        assert_eq!(
            parse_encoded_pointer(encoding, &parameters, &mut rest),
            Err(Error::UnsupportedPointerEncoding)
        );
    }

    #[test]
    fn test_parse_encoded_pointer_indirect() {
        let expected_rest = [1, 2, 3, 4];
        let encoding = constants::DW_EH_PE_indirect;

        let input = Section::with_endian(Endian::Little)
            .L32(0x1234_5678)
            .append_bytes(&expected_rest);
        let input = input.get_contents().unwrap();
        let input = EndianSlice::new(&input, LittleEndian);
        let mut rest = input;

        let parameters = PointerEncodingParameters {
            bases: &SectionBaseAddresses::default(),
            func_base: None,
            address_size: 4,
            section: &input,
        };
        assert_eq!(
            parse_encoded_pointer(encoding, &parameters, &mut rest),
            Ok(Pointer::Indirect(0x1234_5678))
        );
        assert_eq!(rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_unwind_context_reuse() {
        fn unwind_one(ctx: &mut UnwindContext<usize>, data: &[u8]) {
            let debug_frame = DebugFrame::new(data, NativeEndian);
            let bases = Default::default();
            let result = debug_frame.unwind_info_for_address(
                &bases,
                ctx,
                0xbadb_ad99,
                DebugFrame::cie_from_offset,
            );
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), Error::NoUnwindInfoForAddress);
        }

        // Use the same context for two different data lifetimes.
        let mut ctx: UnwindContext<usize> = UnwindContext::new();
        {
            let data1 = vec![];
            unwind_one(&mut ctx, &data1);
        }
        {
            let data2 = vec![];
            unwind_one(&mut ctx, &data2);
        }
    }
}
