#[cfg(test)]
mod tests {
    use super::*;
    use crate::{test_util::GimliSectionMethods, DebugStr, LittleEndian};
    use test_assembler::{Endian, Label, LabelMaker, Section};

    #[test]
    fn test_get_macinfo() {
        let position = Label::new();

        // Create a test section with some macinfo entries
        let section = Section::with_endian(Endian::Little)
            .set_start_const(0)
            .mark(&position)
            .D8(crate::DW_MACINFO_define.0)
            .uleb(0) // line number: 0 - defined on the compiler command line
            .append_bytes(b"__STDC__ 1\0")
            .D8(crate::DW_MACINFO_define.0)
            .uleb(1) // line number: 1 - defined in the source file
            .append_bytes(b"__GNUC__ 1\0")
            .D8(crate::DW_MACINFO_undef.0)
            .uleb(2) // line number: 2 - undefined in the source file
            .append_bytes(b"__GNUC__\0")
            .D8(crate::DW_MACINFO_start_file.0)
            .uleb(3) // line number: 3 - start of file
            .uleb(4) // file number index: 4 - index into the line number table
            .D8(crate::DW_MACINFO_end_file.0) // end of file
            .D8(crate::DW_MACINFO_vendor_ext.0)
            .uleb(5) // numeric constant: 5 - vendor specific
            .append_bytes(b"foo\0")
            .D8(0); // end of unit

        // Create a DebugMacinfo instance from the section
        let section = section.get_contents().unwrap();
        let debug_macinfo = DebugMacinfo::from(EndianSlice::new(&section, LittleEndian));

        let offset = position.value().unwrap() as usize;

        let mut iter = debug_macinfo
            .get_macinfo(DebugMacinfoOffset(offset))
            .unwrap();

        // Test getting macinfo entries
        let entry = iter.next().unwrap().unwrap();
        assert!(
            matches!(entry, MacroEntry::Define { line: 0, text: MacroString::Direct(text) } if text.slice() == b"__STDC__ 1")
        );

        let entry = iter.next().unwrap().unwrap();
        assert!(
            matches!(entry, MacroEntry::Define { line: 1, text: MacroString::Direct(text) } if text.slice() == b"__GNUC__ 1")
        );

        let entry = iter.next().unwrap().unwrap();
        assert!(
            matches!(entry, MacroEntry::Undef { line: 2, name: MacroString::Direct(name) } if name.slice() == b"__GNUC__")
        );

        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(entry, MacroEntry::StartFile { line: 3, file: 4 }));

        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(entry, MacroEntry::EndFile));

        let entry = iter.next().unwrap().unwrap();
        assert!(
            matches!(entry, MacroEntry::VendorExt { numeric: 5, string } if string.slice() == b"foo")
        );

        assert_eq!(iter.next(), Ok(None));
    }

    #[test]
    fn get_macros_1() {
        let position = Label::new();

        // The test data is originally from the DWARF v5 standard, appendix D.16
        // 1) Figure D.71, simple DWARF encoding
        let section = Section::with_endian(Endian::Little)
            .set_start_const(0)
            .mark(&position)
            .D16(5) // Dwarf version
            .D8(0b0000_0010) // Flags: offset_size = 0 (32-bit), debug_line_offset = 1, opcode_operands_table = 0
            .D32(0) // debug line offset
            .D8(crate::DW_MACRO_start_file.0) // start file: "a.c"
            .uleb(0) // line number
            .uleb(0) // file number
            .D8(crate::DW_MACRO_start_file.0) // start file: "a.h"
            .uleb(1) // line number
            .uleb(1) // file number
            .D8(crate::DW_MACRO_define.0) // define
            .uleb(1) // line number
            .append_bytes(b"LONGER_MACRO 1\0") // macro name
            .D8(crate::DW_MACRO_define.0) // define
            .uleb(2) // line number
            .append_bytes(b"B 2\0") // macro name
            .D8(crate::DW_MACRO_start_file.0) // start file: "b.h"
            .uleb(3) // line number
            .uleb(2) // file number
            .D8(crate::DW_MACRO_undef.0) // undef
            .uleb(1) // line number
            .append_bytes(b"B\0") // macro name
            .D8(crate::DW_MACRO_define.0) // define
            .uleb(2) // line number
            .append_bytes(b"D 3\0") // macro name
            .D8(crate::DW_MACRO_define.0) // define
            .uleb(3) // line number
            .append_bytes(b"FUNCTION_LIKE_MACRO(x) 4+x\0") // macro name
            .D8(crate::DW_MACRO_end_file.0) // end file: "b.h" -> "a.h"
            .D8(crate::DW_MACRO_define.0) // define
            .uleb(4) // line number
            .append_bytes(b"B 3\0") // macro name
            .D8(crate::DW_MACRO_end_file.0) // end file: "a.h" -> "a.c"
            .D8(crate::DW_MACRO_define.0) // define
            .uleb(2) // line number
            .append_bytes(b"FUNCTION_LIKE_MACRO(x) 4+x\0") // macro name
            .D8(crate::DW_MACRO_start_file.0) // start file: "b.h"
            .uleb(3) // line number
            .uleb(2) // file number
            .D8(crate::DW_MACRO_undef.0) // undef
            .uleb(1) // line number
            .append_bytes(b"B\0") // macro name
            .D8(crate::DW_MACRO_define.0) // define
            .uleb(2) // line number
            .append_bytes(b"D 3\0") // macro name
            .D8(crate::DW_MACRO_define.0) // define
            .uleb(3) // line number
            .append_bytes(b"FUNCTION_LIKE_MACRO(x) 4+x\0") // macro name
            .D8(crate::DW_MACRO_end_file.0) // end file: "b.h" -> "a.c"
            .D8(crate::DW_MACRO_end_file.0) // end file: "a.c" -> ""
            .D8(0); // end of unit

        // Create a DebugMacro instance from the section
        let section = section.get_contents().unwrap();
        let debug_macro = DebugMacro::from(EndianSlice::new(&section, LittleEndian));

        let offset = position.value().unwrap() as usize;

        let mut iter = debug_macro.get_macros(DebugMacroOffset(offset)).unwrap();
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(entry, MacroEntry::StartFile { line: 0, file: 0 }));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(entry, MacroEntry::StartFile { line: 1, file: 1 }));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(
            entry,
            MacroEntry::Define {
                line: 1, text: MacroString::Direct(text)
            } if text.slice() == b"LONGER_MACRO 1"
        ));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(
            entry,
            MacroEntry::Define {
                line: 2, text: MacroString::Direct(text)
            } if text.slice() == b"B 2"
        ));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(entry, MacroEntry::StartFile { line: 3, file: 2 }));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(
            entry,
            MacroEntry::Undef {
                line: 1, name: MacroString::Direct(name)
            } if name.slice() == b"B"
        ));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(
            entry,
            MacroEntry::Define {
                line: 2, text: MacroString::Direct(text)
            } if text.slice() == b"D 3"
        ));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(
            entry,
            MacroEntry::Define {
                line: 3, text: MacroString::Direct(text)
            } if text.slice() == b"FUNCTION_LIKE_MACRO(x) 4+x"
        ));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(entry, MacroEntry::EndFile));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(
            entry,
            MacroEntry::Define {
                line: 4, text: MacroString::Direct(text)
            } if text.slice() == b"B 3"
        ));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(entry, MacroEntry::EndFile));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(
            entry,
            MacroEntry::Define {
                line: 2, text: MacroString::Direct(text)
            } if text.slice() == b"FUNCTION_LIKE_MACRO(x) 4+x"
        ));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(entry, MacroEntry::StartFile { line: 3, file: 2 }));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(
            entry,
            MacroEntry::Undef {
                line: 1, name: MacroString::Direct(name)
            } if name.slice() == b"B"
        ));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(
            entry,
            MacroEntry::Define {
                line: 2, text: MacroString::Direct(text)
            } if text.slice() == b"D 3"
        ));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(
            entry,
            MacroEntry::Define {
                line: 3, text: MacroString::Direct(text)
            } if text.slice() == b"FUNCTION_LIKE_MACRO(x) 4+x"
        ));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(entry, MacroEntry::EndFile));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(entry, MacroEntry::EndFile));
        assert_eq!(iter.next(), Ok(None));
    }

    #[test]
    fn get_macros_2() {
        let str_0 = Label::new();
        let str_1 = Label::new();
        let macro_unit_0 = Label::new();
        let macro_unit_1 = Label::new();
        let macro_unit_2 = Label::new();

        // The test data is originally from the DWARF v5 standard, appendix D.16
        // 2) Figure D.72, shareable DWARF encoding
        let str_section = Section::with_endian(Endian::Little)
            .set_start_const(0)
            .mark(&str_0)
            .append_bytes(b"FUNCTION_LIKE_MACRO(x) 4+x\0") // macro name
            .mark(&str_1)
            .append_bytes(b"LONGER_MACRO 1\0"); // macro name

        let macro_section = Section::with_endian(Endian::Little)
            .set_start_const(0)
            //--------------------unit 0----------------------
            .mark(&macro_unit_0) // start of unit 0
            .D16(5) // Dwarf version
            .D8(0b0000_0010) // Flags: offset_size = 0 (32-bit), debug_line_offset = 1, opcode_operands_table = 0
            .D32(0) // debug line offset
            .D8(crate::DW_MACRO_start_file.0) // start file: "a.c"
            .uleb(0) // line number
            .uleb(0) // file number
            .D8(crate::DW_MACRO_start_file.0) // start file: "a.h"
            .uleb(1) // line number
            .uleb(1) // file number
            .D8(crate::DW_MACRO_import.0) // import unit 1
            .L32(macro_unit_1.clone()) // debug line offset to unit 1
            .D8(crate::DW_MACRO_start_file.0) // start file: "b.h"
            .uleb(3) // line number
            .uleb(2) // file number
            .D8(crate::DW_MACRO_import.0) // import unit 2
            .L32(macro_unit_2.clone()) // debug line offset to unit 2
            .D8(crate::DW_MACRO_end_file.0) // end file: "b.h" -> "a.h"
            .D8(crate::DW_MACRO_define.0) // define
            .uleb(4) // line number
            .append_bytes(b"B 3\0") // macro name
            .D8(crate::DW_MACRO_end_file.0) // end file: "a.h" -> "a.c"
            .D8(crate::DW_MACRO_define_strp.0) // define: "FUNCTION_LIKE_MACRO(x) 4+x"
            .uleb(2) // line number
            .D32(0) // macro name offset in the string table
            .D8(crate::DW_MACRO_start_file.0) // start file: "b.h"
            .uleb(3) // line number
            .uleb(2) // file number
            .D8(crate::DW_MACRO_import.0) // import unit 2
            .L32(&macro_unit_2) // debug line offset to unit 2
            .D8(crate::DW_MACRO_end_file.0) // end file: "b.h" -> "a.c"
            .D8(crate::DW_MACRO_end_file.0) // end file: "a.c" -> ""
            .D8(0)
            //--------------------unit 1----------------------
            .mark(&macro_unit_1) // start of unit 1
            .D16(5) // Dwarf version
            .D8(0b0000_0000) // Flags: offset_size = 0 (32-bit), debug_line_offset = 0, opcode_operands_table = 0
            .D8(crate::DW_MACRO_define_strp.0) // define strp: "LONGER_MACRO 1"
            .uleb(1) // line number
            .L32(str_0.clone()) // macro name offset in the string table
            .D8(crate::DW_MACRO_define.0) // define: "B 2"
            .uleb(2) // line number
            .append_bytes(b"B 2\0") // macro name
            .D8(0) // end of unit
            //---------------------unit 2---------------------
            .mark(&macro_unit_2) // start of unit 2
            .D16(5) // Dwarf version
            .D8(0b0000_0000) // Flags: offset_size = 0 (32-bit), debug_line_offset = 0, opcode_operands_table = 0
            .D8(crate::DW_MACRO_undef.0) // undef: "B"
            .uleb(1) // line number
            .append_bytes(b"B\0") // macro name
            .D8(crate::DW_MACRO_define.0) // define: "D 3"
            .uleb(2) // line number
            .append_bytes(b"D 3\0") // macro name
            .D8(crate::DW_MACRO_define_strp.0) // define strp: "FUNCTION_LIKE_MACRO(x) 4+x"
            .uleb(2) // line number
            .L32(str_1.clone()) // macro name offset in the string table
            .D8(0); // end of unit

        // Create a DebugMacro instance from the section
        let str_section = str_section.get_contents().unwrap();
        let debug_str = DebugStr::from(EndianSlice::new(&str_section, LittleEndian));

        // Create a DebugMacro instance from the section
        let macro_section = macro_section.get_contents().unwrap();
        let debug_macro = DebugMacro::from(EndianSlice::new(&macro_section, LittleEndian));

        // check the content of macro unit 0
        let offset = macro_unit_0.value().unwrap() as usize;
        let mut iter = debug_macro.get_macros(DebugMacroOffset(offset)).unwrap();
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(entry, MacroEntry::StartFile { line: 0, file: 0 }));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(entry, MacroEntry::StartFile { line: 1, file: 1 }));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(
            entry,
            MacroEntry::Import { offset } if offset.0 == macro_unit_1.value().unwrap() as usize
        ));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(entry, MacroEntry::StartFile { line: 3, file: 2 }));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(
            entry,
            MacroEntry::Import { offset } if offset.0 == macro_unit_2.value().unwrap() as usize
        ));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(entry, MacroEntry::EndFile));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(
            entry,
            MacroEntry::Define {
                line: 4, text: MacroString::Direct(text)
            } if text.slice() == b"B 3"
        ));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(entry, MacroEntry::EndFile));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(
            entry,
            MacroEntry::Define {
                line: 2, text: MacroString::StringPointer(text_offset)
            } if text_offset.0 == str_0.value().unwrap() as usize
        ));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(entry, MacroEntry::StartFile { line: 3, file: 2 }));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(
            entry,
            MacroEntry::Import { offset } if offset.0 == macro_unit_2.value().unwrap() as usize
        ));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(entry, MacroEntry::EndFile));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(entry, MacroEntry::EndFile));
        assert_eq!(iter.next(), Ok(None));

        // check the content of macro unit 1
        let offset = macro_unit_1.value().unwrap() as usize;
        let mut iter = debug_macro.get_macros(DebugMacroOffset(offset)).unwrap();
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(
            entry,
            MacroEntry::Define {
                line: 1, text: MacroString::StringPointer(text_offset)
            } if text_offset.0 == str_0.value().unwrap() as usize
        ));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(
            entry,
            MacroEntry::Define {
                line: 2, text: MacroString::Direct(text)
            } if text.slice() == b"B 2"
        ));
        assert_eq!(iter.next(), Ok(None));

        // check the content of macro unit 2
        let offset = macro_unit_2.value().unwrap() as usize;
        let mut iter = debug_macro.get_macros(DebugMacroOffset(offset)).unwrap();
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(
            entry,
            MacroEntry::Undef {
                line: 1, name: MacroString::Direct(name)
            } if name.slice() == b"B"
        ));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(
            entry,
            MacroEntry::Define {
                line: 2, text: MacroString::Direct(text)
            } if text.slice() == b"D 3"
        ));
        let entry = iter.next().unwrap().unwrap();
        assert!(matches!(
            entry,
            MacroEntry::Define {
                line: 2, text: MacroString::StringPointer(text_offset)
            } if text_offset.0 == str_1.value().unwrap() as usize
        ));
        assert_eq!(iter.next(), Ok(None));

        // check the content of the string table
        let text_offset = DebugStrOffset(str_0.value().unwrap() as usize);
        assert_eq!(
            debug_str.get_str(text_offset).unwrap().slice(),
            b"FUNCTION_LIKE_MACRO(x) 4+x"
        );
        let text_offset = DebugStrOffset(str_1.value().unwrap() as usize);
        assert_eq!(
            debug_str.get_str(text_offset).unwrap().slice(),
            b"LONGER_MACRO 1"
        );
    }
}
