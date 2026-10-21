#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::constants;
    use crate::endianity::LittleEndian;
    use crate::read::{EndianSlice, Error};
    use crate::test_util::GimliSectionMethods;
    #[cfg(target_pointer_width = "32")]
    use core::u32;
    use test_assembler::Section;

    pub trait AbbrevSectionMethods {
        fn abbrev(self, code: u64, tag: constants::DwTag, children: constants::DwChildren) -> Self;
        fn abbrev_null(self) -> Self;
        fn abbrev_attr(self, name: constants::DwAt, form: constants::DwForm) -> Self;
        fn abbrev_attr_implicit_const(self, name: constants::DwAt, value: i64) -> Self;
        fn abbrev_attr_null(self) -> Self;
    }

    impl AbbrevSectionMethods for Section {
        fn abbrev(self, code: u64, tag: constants::DwTag, children: constants::DwChildren) -> Self {
            self.uleb(code).uleb(tag.0.into()).D8(children.0)
        }

        fn abbrev_null(self) -> Self {
            self.D8(0)
        }

        fn abbrev_attr(self, name: constants::DwAt, form: constants::DwForm) -> Self {
            self.uleb(name.0.into()).uleb(form.0.into())
        }

        fn abbrev_attr_implicit_const(self, name: constants::DwAt, value: i64) -> Self {
            self.uleb(name.0.into())
                .uleb(constants::DW_FORM_implicit_const.0.into())
                .sleb(value)
        }

        fn abbrev_attr_null(self) -> Self {
            self.D8(0).D8(0)
        }
    }

    #[test]
    fn test_debug_abbrev_ok() {
        let extra_start = [1, 2, 3, 4];
        let expected_rest = [5, 6, 7, 8];
        #[rustfmt::skip]
        let buf = Section::new()
            .append_bytes(&extra_start)
            .abbrev(2, constants::DW_TAG_subprogram, constants::DW_CHILDREN_no)
                .abbrev_attr(constants::DW_AT_name, constants::DW_FORM_string)
                .abbrev_attr_null()
            .abbrev(1, constants::DW_TAG_compile_unit, constants::DW_CHILDREN_yes)
                .abbrev_attr(constants::DW_AT_producer, constants::DW_FORM_strp)
                .abbrev_attr(constants::DW_AT_language, constants::DW_FORM_data2)
                .abbrev_attr_null()
            .abbrev_null()
            .append_bytes(&expected_rest)
            .get_contents()
            .unwrap();

        let abbrev1 = Abbreviation::new(
            1,
            constants::DW_TAG_compile_unit,
            constants::DW_CHILDREN_yes,
            vec![
                AttributeSpecification::new(
                    constants::DW_AT_producer,
                    constants::DW_FORM_strp,
                    None,
                ),
                AttributeSpecification::new(
                    constants::DW_AT_language,
                    constants::DW_FORM_data2,
                    None,
                ),
            ]
            .into(),
        );

        let abbrev2 = Abbreviation::new(
            2,
            constants::DW_TAG_subprogram,
            constants::DW_CHILDREN_no,
            vec![AttributeSpecification::new(
                constants::DW_AT_name,
                constants::DW_FORM_string,
                None,
            )]
            .into(),
        );

        let debug_abbrev = DebugAbbrev::new(&buf, LittleEndian);
        let debug_abbrev_offset = DebugAbbrevOffset(extra_start.len());
        let abbrevs = debug_abbrev
            .abbreviations(debug_abbrev_offset)
            .expect("Should parse abbreviations");
        assert_eq!(abbrevs.get(1), Some(&abbrev1));
        assert_eq!(abbrevs.get(2), Some(&abbrev2));
    }

    #[test]
    fn test_abbreviations_insert() {
        fn abbrev(code: u16) -> Abbreviation {
            Abbreviation::new(
                code.into(),
                constants::DwTag(code),
                constants::DW_CHILDREN_no,
                vec![].into(),
            )
        }

        fn assert_abbrev(abbrevs: &Abbreviations, code: u16) {
            let abbrev = abbrevs.get(code.into()).unwrap();
            assert_eq!(abbrev.tag(), constants::DwTag(code));
        }

        // Sequential insert.
        let mut abbrevs = Abbreviations::empty();
        abbrevs.insert(abbrev(1)).unwrap();
        abbrevs.insert(abbrev(2)).unwrap();
        assert_eq!(abbrevs.vec.len(), 2);
        assert!(abbrevs.map.is_empty());
        assert_abbrev(&abbrevs, 1);
        assert_abbrev(&abbrevs, 2);

        // Out of order insert.
        let mut abbrevs = Abbreviations::empty();
        abbrevs.insert(abbrev(2)).unwrap();
        abbrevs.insert(abbrev(3)).unwrap();
        assert!(abbrevs.vec.is_empty());
        assert_abbrev(&abbrevs, 2);
        assert_abbrev(&abbrevs, 3);

        // Mixed order insert.
        let mut abbrevs = Abbreviations::empty();
        abbrevs.insert(abbrev(1)).unwrap();
        abbrevs.insert(abbrev(3)).unwrap();
        abbrevs.insert(abbrev(2)).unwrap();
        assert_eq!(abbrevs.vec.len(), 2);
        assert_abbrev(&abbrevs, 1);
        assert_abbrev(&abbrevs, 2);
        assert_abbrev(&abbrevs, 3);

        // Duplicate code in vec.
        let mut abbrevs = Abbreviations::empty();
        abbrevs.insert(abbrev(1)).unwrap();
        abbrevs.insert(abbrev(2)).unwrap();
        assert_eq!(abbrevs.insert(abbrev(1)), Err(()));
        assert_eq!(abbrevs.insert(abbrev(2)), Err(()));

        // Duplicate code in map when adding to map.
        let mut abbrevs = Abbreviations::empty();
        abbrevs.insert(abbrev(2)).unwrap();
        assert_eq!(abbrevs.insert(abbrev(2)), Err(()));

        // Duplicate code in map when adding to vec.
        let mut abbrevs = Abbreviations::empty();
        abbrevs.insert(abbrev(2)).unwrap();
        abbrevs.insert(abbrev(1)).unwrap();
        assert_eq!(abbrevs.insert(abbrev(2)), Err(()));

        // 32-bit usize conversions.
        let mut abbrevs = Abbreviations::empty();
        abbrevs.insert(abbrev(2)).unwrap();
    }

    #[test]
    #[cfg(target_pointer_width = "32")]
    fn test_abbreviations_insert_32() {
        fn abbrev(code: u64) -> Abbreviation {
            Abbreviation::new(
                code,
                constants::DwTag(code as u16),
                constants::DW_CHILDREN_no,
                vec![].into(),
            )
        }

        fn assert_abbrev(abbrevs: &Abbreviations, code: u64) {
            let abbrev = abbrevs.get(code).unwrap();
            assert_eq!(abbrev.tag(), constants::DwTag(code as u16));
        }

        let mut abbrevs = Abbreviations::empty();
        abbrevs.insert(abbrev(1)).unwrap();

        let wrap_code = (u32::MAX as u64 + 1) + 1;
        // `get` should not treat the wrapped code as `1`.
        assert_eq!(abbrevs.get(wrap_code), None);
        // `insert` should not treat the wrapped code as `1`.
        abbrevs.insert(abbrev(wrap_code)).unwrap();
        assert_abbrev(&abbrevs, 1);
        assert_abbrev(&abbrevs, wrap_code);
    }

    #[test]
    fn test_parse_abbreviations_ok() {
        let expected_rest = [1, 2, 3, 4];
        #[rustfmt::skip]
        let buf = Section::new()
            .abbrev(2, constants::DW_TAG_subprogram, constants::DW_CHILDREN_no)
                .abbrev_attr(constants::DW_AT_name, constants::DW_FORM_string)
                .abbrev_attr_null()
            .abbrev(1, constants::DW_TAG_compile_unit, constants::DW_CHILDREN_yes)
                .abbrev_attr(constants::DW_AT_producer, constants::DW_FORM_strp)
                .abbrev_attr(constants::DW_AT_language, constants::DW_FORM_data2)
                .abbrev_attr_null()
            .abbrev_null()
            .append_bytes(&expected_rest)
            .get_contents()
            .unwrap();
        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        let abbrev1 = Abbreviation::new(
            1,
            constants::DW_TAG_compile_unit,
            constants::DW_CHILDREN_yes,
            vec![
                AttributeSpecification::new(
                    constants::DW_AT_producer,
                    constants::DW_FORM_strp,
                    None,
                ),
                AttributeSpecification::new(
                    constants::DW_AT_language,
                    constants::DW_FORM_data2,
                    None,
                ),
            ]
            .into(),
        );

        let abbrev2 = Abbreviation::new(
            2,
            constants::DW_TAG_subprogram,
            constants::DW_CHILDREN_no,
            vec![AttributeSpecification::new(
                constants::DW_AT_name,
                constants::DW_FORM_string,
                None,
            )]
            .into(),
        );

        let abbrevs = Abbreviations::parse(rest).expect("Should parse abbreviations");
        assert_eq!(abbrevs.get(1), Some(&abbrev1));
        assert_eq!(abbrevs.get(2), Some(&abbrev2));
        assert_eq!(*rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_abbreviations_duplicate() {
        let expected_rest = [1, 2, 3, 4];
        #[rustfmt::skip]
        let buf = Section::new()
            .abbrev(1, constants::DW_TAG_subprogram, constants::DW_CHILDREN_no)
                .abbrev_attr(constants::DW_AT_name, constants::DW_FORM_string)
                .abbrev_attr_null()
            .abbrev(1, constants::DW_TAG_compile_unit, constants::DW_CHILDREN_yes)
                .abbrev_attr(constants::DW_AT_producer, constants::DW_FORM_strp)
                .abbrev_attr(constants::DW_AT_language, constants::DW_FORM_data2)
                .abbrev_attr_null()
            .abbrev_null()
            .append_bytes(&expected_rest)
            .get_contents()
            .unwrap();
        let buf = &mut EndianSlice::new(&buf, LittleEndian);

        match Abbreviations::parse(buf) {
            Err(Error::DuplicateAbbreviationCode) => {}
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    fn test_parse_abbreviation_tag_ok() {
        let buf = [0x01, 0x02];
        let rest = &mut EndianSlice::new(&buf, LittleEndian);
        let tag = Abbreviation::parse_tag(rest).expect("Should parse tag");
        assert_eq!(tag, constants::DW_TAG_array_type);
        assert_eq!(*rest, EndianSlice::new(&buf[1..], LittleEndian));
    }

    #[test]
    fn test_parse_abbreviation_tag_zero() {
        let buf = [0x00];
        let buf = &mut EndianSlice::new(&buf, LittleEndian);
        match Abbreviation::parse_tag(buf) {
            Err(Error::AbbreviationTagZero) => {}
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    fn test_parse_abbreviation_has_children() {
        let buf = [0x00, 0x01, 0x02];
        let rest = &mut EndianSlice::new(&buf, LittleEndian);
        let val = Abbreviation::parse_has_children(rest).expect("Should parse children");
        assert_eq!(val, constants::DW_CHILDREN_no);
        let val = Abbreviation::parse_has_children(rest).expect("Should parse children");
        assert_eq!(val, constants::DW_CHILDREN_yes);
        match Abbreviation::parse_has_children(rest) {
            Err(Error::BadHasChildren) => {}
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    fn test_parse_abbreviation_ok() {
        let expected_rest = [0x01, 0x02, 0x03, 0x04];
        let buf = Section::new()
            .abbrev(1, constants::DW_TAG_subprogram, constants::DW_CHILDREN_no)
            .abbrev_attr(constants::DW_AT_name, constants::DW_FORM_string)
            .abbrev_attr_null()
            .append_bytes(&expected_rest)
            .get_contents()
            .unwrap();
        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        let expect = Some(Abbreviation::new(
            1,
            constants::DW_TAG_subprogram,
            constants::DW_CHILDREN_no,
            vec![AttributeSpecification::new(
                constants::DW_AT_name,
                constants::DW_FORM_string,
                None,
            )]
            .into(),
        ));

        let abbrev = Abbreviation::parse(rest).expect("Should parse abbreviation");
        assert_eq!(abbrev, expect);
        assert_eq!(*rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_abbreviation_implicit_const_ok() {
        let expected_rest = [0x01, 0x02, 0x03, 0x04];
        let buf = Section::new()
            .abbrev(1, constants::DW_TAG_subprogram, constants::DW_CHILDREN_no)
            .abbrev_attr_implicit_const(constants::DW_AT_name, -42)
            .abbrev_attr_null()
            .append_bytes(&expected_rest)
            .get_contents()
            .unwrap();
        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        let expect = Some(Abbreviation::new(
            1,
            constants::DW_TAG_subprogram,
            constants::DW_CHILDREN_no,
            vec![AttributeSpecification::new(
                constants::DW_AT_name,
                constants::DW_FORM_implicit_const,
                Some(-42),
            )]
            .into(),
        ));

        let abbrev = Abbreviation::parse(rest).expect("Should parse abbreviation");
        assert_eq!(abbrev, expect);
        assert_eq!(*rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_abbreviation_implicit_const_no_const() {
        let buf = Section::new()
            .abbrev(1, constants::DW_TAG_subprogram, constants::DW_CHILDREN_no)
            .abbrev_attr(constants::DW_AT_name, constants::DW_FORM_implicit_const)
            .get_contents()
            .unwrap();
        let buf = &mut EndianSlice::new(&buf, LittleEndian);

        match Abbreviation::parse(buf) {
            Err(Error::UnexpectedEof(_)) => {}
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        }
    }

    #[test]
    fn test_parse_null_abbreviation_ok() {
        let expected_rest = [0x01, 0x02, 0x03, 0x04];
        let buf = Section::new()
            .abbrev_null()
            .append_bytes(&expected_rest)
            .get_contents()
            .unwrap();
        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        let abbrev = Abbreviation::parse(rest).expect("Should parse null abbreviation");
        assert!(abbrev.is_none());
        assert_eq!(*rest, EndianSlice::new(&expected_rest, LittleEndian));
    }

    #[test]
    fn test_parse_attribute_form_ok() {
        let buf = [0x01, 0x02];
        let rest = &mut EndianSlice::new(&buf, LittleEndian);
        let tag = AttributeSpecification::parse_form(rest).expect("Should parse form");
        assert_eq!(tag, constants::DW_FORM_addr);
        assert_eq!(*rest, EndianSlice::new(&buf[1..], LittleEndian));
    }

    #[test]
    fn test_parse_attribute_form_zero() {
        let buf = [0x00];
        let buf = &mut EndianSlice::new(&buf, LittleEndian);
        match AttributeSpecification::parse_form(buf) {
            Err(Error::AttributeFormZero) => {}
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    fn test_parse_null_attribute_specification_ok() {
        let buf = [0x00, 0x00, 0x01];
        let rest = &mut EndianSlice::new(&buf, LittleEndian);
        let attr =
            AttributeSpecification::parse(rest).expect("Should parse null attribute specification");
        assert!(attr.is_none());
        assert_eq!(*rest, EndianSlice::new(&buf[2..], LittleEndian));
    }

    #[test]
    fn test_parse_attribute_specifications_name_zero() {
        let buf = [0x00, 0x01, 0x00, 0x00];
        let buf = &mut EndianSlice::new(&buf, LittleEndian);
        match AttributeSpecification::parse(buf) {
            Err(Error::ExpectedZero) => {}
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    fn test_parse_attribute_specifications_form_zero() {
        let buf = [0x01, 0x00, 0x00, 0x00];
        let buf = &mut EndianSlice::new(&buf, LittleEndian);
        match AttributeSpecification::parse(buf) {
            Err(Error::AttributeFormZero) => {}
            otherwise => panic!("Unexpected result: {:?}", otherwise),
        };
    }

    #[test]
    fn test_get_abbrev_zero() {
        let mut abbrevs = Abbreviations::empty();
        abbrevs
            .insert(Abbreviation::new(
                1,
                constants::DwTag(1),
                constants::DW_CHILDREN_no,
                vec![].into(),
            ))
            .unwrap();
        assert!(abbrevs.get(0).is_none());
    }
}
