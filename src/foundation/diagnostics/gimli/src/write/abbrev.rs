use alloc::vec::Vec;
use indexmap::IndexSet;
use std::ops::{Deref, DerefMut};

use crate::common::{DebugAbbrevOffset, SectionId};
use crate::constants;
use crate::write::{Result, Section, Writer};

/// A table of abbreviations that will be stored in a `.debug_abbrev` section.
// Requirements:
// - values are `Abbreviation`
// - insertion returns an abbreviation code for use in writing a DIE
// - inserting a duplicate returns the code of the existing value
#[derive(Debug, Default)]
pub(crate) struct AbbreviationTable {
    abbrevs: IndexSet<Abbreviation>,
}

impl AbbreviationTable {
    /// Add an abbreviation to the table and return its code.
    pub fn add(&mut self, abbrev: Abbreviation) -> u64 {
        let (code, _) = self.abbrevs.insert_full(abbrev);
        // Code must be non-zero
        (code + 1) as u64
    }

    /// Write the abbreviation table to the `.debug_abbrev` section.
    pub fn write<W: Writer>(&self, w: &mut DebugAbbrev<W>) -> Result<()> {
        for (code, abbrev) in self.abbrevs.iter().enumerate() {
            w.write_uleb128((code + 1) as u64)?;
            abbrev.write(w)?;
        }
        // Null abbreviation code
        w.write_u8(0)
    }
}

/// An abbreviation describes the shape of a `DebuggingInformationEntry`'s type:
/// its tag type, whether it has children, and its set of attributes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Abbreviation {
    tag: constants::DwTag,
    has_children: bool,
    attributes: Vec<AttributeSpecification>,
}

impl Abbreviation {
    /// Construct a new `Abbreviation`.
    #[inline]
    pub fn new(
        tag: constants::DwTag,
        has_children: bool,
        attributes: Vec<AttributeSpecification>,
    ) -> Abbreviation {
        Abbreviation {
            tag,
            has_children,
            attributes,
        }
    }

    /// Write the abbreviation to the `.debug_abbrev` section.
    pub fn write<W: Writer>(&self, w: &mut DebugAbbrev<W>) -> Result<()> {
        w.write_uleb128(self.tag.0.into())?;
        w.write_u8(if self.has_children {
            constants::DW_CHILDREN_yes.0
        } else {
            constants::DW_CHILDREN_no.0
        })?;
        for attr in &self.attributes {
            attr.write(w)?;
        }
        // Null name and form
        w.write_u8(0)?;
        w.write_u8(0)
    }
}

/// The description of an attribute in an abbreviated type.
// TODO: support implicit const
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct AttributeSpecification {
    name: constants::DwAt,
    form: constants::DwForm,
}

impl AttributeSpecification {
    /// Construct a new `AttributeSpecification`.
    #[inline]
    pub fn new(name: constants::DwAt, form: constants::DwForm) -> AttributeSpecification {
        AttributeSpecification { name, form }
    }

    /// Write the attribute specification to the `.debug_abbrev` section.
    #[inline]
    pub fn write<W: Writer>(&self, w: &mut DebugAbbrev<W>) -> Result<()> {
        w.write_uleb128(self.name.0.into())?;
        w.write_uleb128(self.form.0.into())
    }
}

define_section!(
    DebugAbbrev,
    DebugAbbrevOffset,
    "A writable `.debug_abbrev` section."
);

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/gimli/unit/src/write/abbrev/tests_108.rs"));
