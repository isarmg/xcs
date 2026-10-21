#[cfg(test)]
mod tests {
    use super::*;
    use gimli::constants;
    use gimli::write::*;
    use gimli::{Encoding, Format};

    #[test]
    fn test_create_instance() {
        let mut unit1 = Unit::new(
            Encoding {
                version: 4,
                address_size: 8,
                format: Format::Dwarf32,
            },
            LineProgram::none(),
        );

        let root_id = unit1.root();
        let child1_id = unit1.add(root_id, constants::DW_TAG_subprogram);
        let child2_id = unit1.add(child1_id, constants::DW_TAG_lexical_block);

        let mut cursor = DebuggingInformationCursor::new(&mut unit1);

        assert!(cursor.current().is_none());
        assert_eq!(cursor.next_dfs().unwrap().id(), root_id);
        assert_eq!(cursor.next_dfs().unwrap().id(), child1_id);
        assert_eq!(cursor.next_dfs().unwrap().id(), child2_id);
        assert!(cursor.next_dfs().is_none());
    }
}
