#[cfg(test)]
#[cfg(feature = "read")]
mod tests {
    use super::*;
    use crate::constants;
    use crate::read;
    use crate::write::EndianVec;
    use crate::LittleEndian;

    #[test]
    fn test_abbreviation_table() {
        let mut abbrevs = AbbreviationTable::default();
        let abbrev1 = Abbreviation::new(
            constants::DW_TAG_subprogram,
            false,
            vec![AttributeSpecification::new(
                constants::DW_AT_name,
                constants::DW_FORM_string,
            )],
        );
        let abbrev2 = Abbreviation::new(
            constants::DW_TAG_compile_unit,
            true,
            vec![
                AttributeSpecification::new(constants::DW_AT_producer, constants::DW_FORM_strp),
                AttributeSpecification::new(constants::DW_AT_language, constants::DW_FORM_data2),
            ],
        );
        let code1 = abbrevs.add(abbrev1.clone());
        assert_eq!(code1, 1);
        let code2 = abbrevs.add(abbrev2.clone());
        assert_eq!(code2, 2);
        assert_eq!(abbrevs.add(abbrev1.clone()), code1);
        assert_eq!(abbrevs.add(abbrev2.clone()), code2);

        let mut debug_abbrev = DebugAbbrev::from(EndianVec::new(LittleEndian));
        let debug_abbrev_offset = debug_abbrev.offset();
        assert_eq!(debug_abbrev_offset, DebugAbbrevOffset(0));
        abbrevs.write(&mut debug_abbrev).unwrap();
        assert_eq!(debug_abbrev.offset(), DebugAbbrevOffset(17));

        let read_debug_abbrev = read::DebugAbbrev::new(debug_abbrev.slice(), LittleEndian);
        let read_abbrevs = read_debug_abbrev
            .abbreviations(debug_abbrev_offset)
            .unwrap();

        let read_abbrev1 = read_abbrevs.get(code1).unwrap();
        assert_eq!(abbrev1.tag, read_abbrev1.tag());
        assert_eq!(abbrev1.has_children, read_abbrev1.has_children());
        assert_eq!(abbrev1.attributes.len(), read_abbrev1.attributes().len());
        assert_eq!(
            abbrev1.attributes[0].name,
            read_abbrev1.attributes()[0].name()
        );
        assert_eq!(
            abbrev1.attributes[0].form,
            read_abbrev1.attributes()[0].form()
        );

        let read_abbrev2 = read_abbrevs.get(code2).unwrap();
        assert_eq!(abbrev2.tag, read_abbrev2.tag());
        assert_eq!(abbrev2.has_children, read_abbrev2.has_children());
        assert_eq!(abbrev2.attributes.len(), read_abbrev2.attributes().len());
        assert_eq!(
            abbrev2.attributes[0].name,
            read_abbrev2.attributes()[0].name()
        );
        assert_eq!(
            abbrev2.attributes[0].form,
            read_abbrev2.attributes()[0].form()
        );
        assert_eq!(
            abbrev2.attributes[1].name,
            read_abbrev2.attributes()[1].name()
        );
        assert_eq!(
            abbrev2.attributes[1].form,
            read_abbrev2.attributes()[1].form()
        );
    }
}
