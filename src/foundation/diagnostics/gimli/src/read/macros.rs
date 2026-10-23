use crate::common::{DebugMacinfoOffset, SectionId};
use crate::endianity::Endianity;
use crate::read::{EndianSlice, Reader, ReaderOffset, Section, UnitRef};
use crate::{
    constants, DebugLineOffset, DebugMacroOffset, DebugStrOffset, DebugStrOffsetsIndex, DwMacinfo,
    DwMacro, Error, Format, Result,
};

/// The raw contents of the `.debug_macinfo` section.
#[derive(Debug, Default, Clone, Copy)]
pub struct DebugMacinfo<R> {
    pub(crate) section: R,
}

impl<'input, Endian> DebugMacinfo<EndianSlice<'input, Endian>>
where
    Endian: Endianity,
{
    /// Construct a new `DebugMacinfo` instance from the data in the `.debug_macinfo`
    /// section.
    ///
    /// It is the caller's responsibility to read the `.debug_macinfo` section and
    /// present it as a `&[u8]` slice. That means using some ELF loader on
    /// Linux, a Mach-O loader on macOS, etc.
    ///
    /// ```
    /// use gimli::{DebugMacinfo, LittleEndian};
    ///
    /// # let buf = [1, 0, 95, 95, 83, 84, 68, 67, 95, 95, 32, 49, 0];
    /// # let read_section_somehow = || &buf;
    /// let debug_str = DebugMacinfo::new(read_section_somehow(), LittleEndian);
    /// ```
    pub fn new(macinfo_section: &'input [u8], endian: Endian) -> Self {
        Self::from(EndianSlice::new(macinfo_section, endian))
    }
}

impl<R: Reader> DebugMacinfo<R> {
    /// Look up a macro reference the `.debug_macinfo` section by DebugMacinfoOffset.
    ///
    /// A macinfo offset points to a list of macro information entries in the `.debug_macinfo` section.
    /// To handle this, the function returns an iterator.
    ///
    /// ```
    /// use gimli::{DebugMacinfo, DebugMacinfoOffset, LittleEndian};
    ///
    /// # fn main() -> Result<(), gimli::Error> {
    /// # let buf = [1, 0, 95, 95, 83, 84, 68, 67, 95, 95, 32, 49, 0, 0];
    /// # let offset = DebugMacinfoOffset(0);
    /// # let read_section_somehow = || &buf;
    /// # let debug_macinfo_offset_somehow = || offset;
    /// let debug_macinfo = DebugMacinfo::new(read_section_somehow(), LittleEndian);
    /// let mut iter = debug_macinfo.get_macinfo(debug_macinfo_offset_somehow())?;
    /// while let Some(macinfo) = iter.next()? {
    ///     println!("Found macro info {:?}", macinfo);
    /// }
    /// # Ok(()) }
    /// ```
    pub fn get_macinfo(&self, offset: DebugMacinfoOffset<R::Offset>) -> Result<MacroIter<R>> {
        let mut input = self.section.clone();
        input.skip(offset.0)?;
        Ok(MacroIter {
            input,
            format: Format::Dwarf32,
            is_macro: false,
        })
    }
}

impl<T> DebugMacinfo<T> {
    /// Create a `DebugMacinfo` section that references the data in `self`.
    ///
    /// This is useful when `R` implements `Reader` but `T` does not.
    ///
    /// Used by `DwarfSections::borrow`.
    pub fn borrow<'a, F, R>(&'a self, mut borrow: F) -> DebugMacinfo<R>
    where
        F: FnMut(&'a T) -> R,
    {
        borrow(&self.section).into()
    }
}

impl<R> Section<R> for DebugMacinfo<R> {
    fn id() -> SectionId {
        SectionId::DebugMacinfo
    }

    fn reader(&self) -> &R {
        &self.section
    }
}

impl<R> From<R> for DebugMacinfo<R> {
    fn from(macinfo_section: R) -> Self {
        DebugMacinfo {
            section: macinfo_section,
        }
    }
}

/// The raw contents of the `.debug_macro` section.
#[derive(Debug, Default, Clone, Copy)]
pub struct DebugMacro<R> {
    pub(crate) section: R,
}

impl<'input, Endian> DebugMacro<EndianSlice<'input, Endian>>
where
    Endian: Endianity,
{
    /// Construct a new `DebugMacro` instance from the data in the `.debug_macro`
    /// section.
    ///
    /// It is the caller's responsibility to read the `.debug_macro` section and
    /// present it as a `&[u8]` slice. That means using some ELF loader on
    /// Linux, a Mach-O loader on macOS, etc.
    ///
    /// ```
    /// use gimli::{DebugMacro, LittleEndian};
    ///
    /// # let buf = [1, 0, 95, 95, 83, 84, 68, 67, 95, 95, 32, 49, 0];
    /// # let read_section_somehow = || &buf;
    /// let debug_str = DebugMacro::new(read_section_somehow(), LittleEndian);
    /// ```
    pub fn new(macro_section: &'input [u8], endian: Endian) -> Self {
        Self::from(EndianSlice::new(macro_section, endian))
    }
}

impl<R: Reader> DebugMacro<R> {
    /// Look up a macro reference the `.debug_macinfo` section by DebugMacroOffset.
    ///
    /// A macinfo offset points to a list of macro information entries in the `.debug_macinfo` section.
    /// To handle this, the function returns an iterator.
    ///
    /// ```
    /// use gimli::{DebugMacro, DebugMacroOffset, LittleEndian};
    ///
    /// # fn main() -> Result<(), gimli::Error> {
    /// # let buf = [0x05, 0x00, 0x00, 0x01, 0x00, 0x5f, 0x5f, 0x53, 0x54, 0x44, 0x43, 0x5f, 0x5f, 0x20, 0x31, 0x00, 0x00];
    /// # let offset = DebugMacroOffset(0);
    /// # let read_section_somehow = || &buf;
    /// # let debug_macro_offset_somehow = || offset;
    /// let debug_macro = DebugMacro::new(read_section_somehow(), LittleEndian);
    /// let mut iter = debug_macro.get_macros(debug_macro_offset_somehow())?;
    /// while let Some(cur_macro) = iter.next()? {
    ///     println!("Found macro info {:?}", cur_macro);
    /// }
    /// # Ok(()) }
    /// ```
    pub fn get_macros(&self, offset: DebugMacroOffset<R::Offset>) -> Result<MacroIter<R>> {
        let mut input = self.section.clone();
        input.skip(offset.0)?;
        let header = MacroUnitHeader::parse(&mut input)?;
        Ok(MacroIter {
            input,
            format: header.format(),
            is_macro: true,
        })
    }
}

impl<T> DebugMacro<T> {
    /// Create a `DebugMacro` section that references the data in `self`.
    ///
    /// This is useful when `R` implements `Reader` but `T` does not.
    ///
    /// Used by `DwarfSections::borrow`.
    pub fn borrow<'a, F, R>(&'a self, mut borrow: F) -> DebugMacro<R>
    where
        F: FnMut(&'a T) -> R,
    {
        borrow(&self.section).into()
    }
}

impl<R> Section<R> for DebugMacro<R> {
    fn id() -> SectionId {
        SectionId::DebugMacro
    }

    fn reader(&self) -> &R {
        &self.section
    }
}

impl<R> From<R> for DebugMacro<R> {
    fn from(macro_section: R) -> Self {
        DebugMacro {
            section: macro_section,
        }
    }
}

#[derive(Debug, Clone)]
struct MacroUnitHeader<R: Reader> {
    /// The version of the macro unit header. At the moment only version 5 is defined.
    _version: u16,
    flags: u8,
    _debug_line_offset: DebugLineOffset<R::Offset>,
}

impl<R: Reader> MacroUnitHeader<R> {
    const OFFSET_SIZE_FLAG: u8 = 0b0000_0001;
    const DEBUG_LINE_OFFSET_FLAG: u8 = 0b0000_0010;
    const OPCODE_OPERANDS_TABLE_FLAG: u8 = 0b0000_0100;

    fn parse(input: &mut R) -> Result<Self> {
        let version = input.read_u16()?;
        let flags = input.read_u8()?;
        let format = if flags & Self::OFFSET_SIZE_FLAG == 0 {
            Format::Dwarf32
        } else {
            Format::Dwarf64
        };
        let _debug_line_offset = if flags & Self::DEBUG_LINE_OFFSET_FLAG != 0 {
            DebugLineOffset(input.read_offset(format)?)
        } else {
            DebugLineOffset(R::Offset::from_u64(0)?)
        };
        // if the opcode operands table flag is set, there is a table in the header which currently isn't parsed
        if flags & Self::OPCODE_OPERANDS_TABLE_FLAG != 0 {
            return Err(Error::UnsupportedOpcodeOperandsTable);
        }
        Ok(MacroUnitHeader {
            _version: version,
            flags,
            _debug_line_offset,
        })
    }

    fn format(&self) -> Format {
        if self.flags & Self::OFFSET_SIZE_FLAG == 0 {
            Format::Dwarf32
        } else {
            Format::Dwarf64
        }
    }
}

/// A string in a macro entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MacroString<R, Offset = <R as Reader>::Offset>
where
    R: Reader<Offset = Offset>,
    Offset: ReaderOffset,
{
    /// The string is directly embedded in the macro entry
    Direct(R),
    /// The macro refers to a string in the `.debug_str` section using a `DebugStrOffset`.
    StringPointer(DebugStrOffset<Offset>),
    /// The macro contains an index into an array in the `.debug_str_offsets`
    /// section, which refers to a string in the `.debug_str` section.
    IndirectStringPointer(DebugStrOffsetsIndex<Offset>),
    /// The macro refers to a string in the `.debug_str` section in the supplementary object file
    Supplementary(DebugStrOffset<Offset>),
}

impl<R: Reader> MacroString<R> {
    /// Get the string slice from the macro entry.
    pub fn string(&self, unit: UnitRef<'_, R>) -> Result<R> {
        match self {
            MacroString::Direct(s) => Ok(s.clone()),
            MacroString::StringPointer(offset) => unit.string(*offset),
            MacroString::IndirectStringPointer(index) => {
                let str_offset = unit.string_offset(*index)?;
                unit.string(str_offset)
            }
            MacroString::Supplementary(offset) => unit.sup_string(*offset),
        }
    }
}

/// an Entry in the `.debug_macro` section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MacroEntry<R, Offset = <R as Reader>::Offset>
where
    R: Reader<Offset = Offset>,
    Offset: ReaderOffset,
{
    /// A macro definition.
    Define {
        /// The line number where the macro is defined.
        line: u64,
        /// The text of the macro: The name of the macro followed immediately by any formal
        /// parameters including the surrounding parentheses, followed by the macro definition.
        text: MacroString<R>,
    },
    /// A macro undefinition.
    Undef {
        /// The line number where the macro is undefined.
        line: u64,
        /// The name of the macro without the definition.
        name: MacroString<R>,
    },
    /// The start of a file.
    StartFile {
        /// Line number of the source file on which the inclusion macro directive occurred.
        line: u64,
        /// An index into the line number table of the compilation unit.
        file: u64,
    },
    /// The end of the current included file.
    EndFile,
    /// import a macro unit
    Import {
        /// offset of the macro unit in the `.debug_macro` section
        offset: DebugMacroOffset<Offset>,
    },
    /// import a macro unit from the supplementary object file
    ImportSup {
        /// offset of the macro unit in the `.debug_macro` section of the supplementary object file
        offset: DebugMacroOffset<Offset>,
    },
    /// A vendor-specific extension.
    VendorExt {
        /// A numeric constant, whose meaning is vendor specific.
        numeric: u64,
        /// A string whose meaning is vendor specific.
        string: R,
    },
}

/// Iterator over the entries in the `.debug_macro` section.
#[derive(Clone, Debug)]
pub struct MacroIter<R: Reader> {
    input: R,
    format: Format,
    is_macro: bool,
}

impl<R: Reader> MacroIter<R> {
    /// Advance the iterator to the next entry in the `.debug_macro` section.
    pub fn next(&mut self) -> Result<Option<MacroEntry<R>>> {
        // DW_MACINFO_* and DW_MACRO_* have the same values, so we can use the same parsing logic.
        let macro_type = DwMacro(self.input.read_u8()?);
        match macro_type {
            DwMacro(0) => {
                self.input.empty();
                Ok(None)
            }
            constants::DW_MACRO_define => {
                let line = self.input.read_uleb128()?;
                let text = self.input.read_null_terminated_slice()?;
                Ok(Some(MacroEntry::Define {
                    line,
                    text: MacroString::Direct(text),
                }))
            }
            constants::DW_MACRO_undef => {
                let line = self.input.read_uleb128()?;
                let name = self.input.read_null_terminated_slice()?;
                Ok(Some(MacroEntry::Undef {
                    line,
                    name: MacroString::Direct(name),
                }))
            }
            constants::DW_MACRO_start_file => {
                let line = self.input.read_uleb128()?;
                let file = self.input.read_uleb128()?;
                Ok(Some(MacroEntry::StartFile { line, file }))
            }
            constants::DW_MACRO_end_file => Ok(Some(MacroEntry::EndFile)),
            constants::DW_MACRO_define_strp if self.is_macro => {
                let line = self.input.read_uleb128()?;
                let text_offset = DebugStrOffset(self.input.read_offset(self.format)?);
                Ok(Some(MacroEntry::Define {
                    line,
                    text: MacroString::StringPointer(text_offset),
                }))
            }
            constants::DW_MACRO_undef_strp if self.is_macro => {
                let line = self.input.read_uleb128()?;
                let name_offset = DebugStrOffset(self.input.read_offset(self.format)?);
                Ok(Some(MacroEntry::Undef {
                    line,
                    name: MacroString::StringPointer(name_offset),
                }))
            }
            constants::DW_MACRO_import if self.is_macro => {
                let offset = DebugMacroOffset(self.input.read_offset(self.format)?);
                Ok(Some(MacroEntry::Import { offset }))
            }
            constants::DW_MACRO_define_sup if self.is_macro => {
                let line = self.input.read_uleb128()?;
                let text_offset = DebugStrOffset(self.input.read_offset(self.format)?);
                Ok(Some(MacroEntry::Define {
                    line,
                    text: MacroString::Supplementary(text_offset),
                }))
            }
            constants::DW_MACRO_undef_sup if self.is_macro => {
                let line = self.input.read_uleb128()?;
                let name_offset = DebugStrOffset(self.input.read_offset(self.format)?);
                Ok(Some(MacroEntry::Undef {
                    line,
                    name: MacroString::Supplementary(name_offset),
                }))
            }
            constants::DW_MACRO_import_sup if self.is_macro => {
                let offset = DebugMacroOffset(self.input.read_offset(self.format)?);
                Ok(Some(MacroEntry::ImportSup { offset }))
            }
            constants::DW_MACRO_define_strx if self.is_macro => {
                let line = self.input.read_uleb128()?;
                let index = self.input.read_uleb128().and_then(R::Offset::from_u64)?;
                let text_index = DebugStrOffsetsIndex(index);
                Ok(Some(MacroEntry::Define {
                    line,
                    text: MacroString::IndirectStringPointer(text_index),
                }))
            }
            constants::DW_MACRO_undef_strx if self.is_macro => {
                let line = self.input.read_uleb128()?;
                let index = self.input.read_uleb128().and_then(R::Offset::from_u64)?;
                let name_index = DebugStrOffsetsIndex(index);
                Ok(Some(MacroEntry::Undef {
                    line,
                    name: MacroString::IndirectStringPointer(name_index),
                }))
            }
            _ => {
                if self.is_macro {
                    self.input.empty();
                    Err(Error::InvalidMacroType(macro_type))
                } else if macro_type.0 == constants::DW_MACINFO_vendor_ext.0 {
                    let numeric = self.input.read_uleb128()?;
                    let string = self.input.read_null_terminated_slice()?;
                    Ok(Some(MacroEntry::VendorExt { numeric, string }))
                } else {
                    self.input.empty();
                    Err(Error::InvalidMacinfoType(DwMacinfo(macro_type.0)))
                }
            }
        }
    }
}

#[cfg(feature = "fallible-iterator")]
impl<R: Reader> fallible_iterator::FallibleIterator for MacroIter<R> {
    type Item = MacroEntry<R>;
    type Error = Error;

    fn next(&mut self) -> ::core::result::Result<Option<Self::Item>, Error> {
        MacroIter::next(self)
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/gimli/unit/src/read/macros/tests_450.rs"));
