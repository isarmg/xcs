use alloc::vec::Vec;
use indexmap::IndexSet;
use std::ops::{Deref, DerefMut};

use crate::common::{DebugLineStrOffset, DebugStrOffset, SectionId};
use crate::write::{BaseId, Result, Section, Writer};

// Requirements:
// - values are `[u8]`, null bytes are not allowed
// - insertion returns a fixed id
// - inserting a duplicate returns the id of the existing value
// - able to convert an id to a section offset
// Optional?
// - able to get an existing value given an id
//
// Limitations of current implementation (using IndexSet):
// - inserting requires either an allocation for duplicates,
//   or a double lookup for non-duplicates
// - doesn't preserve offsets when updating an existing `.debug_str` section
//
// Possible changes:
// - calculate offsets as we add values, and use that as the id.
//   This would avoid the need for DebugStrOffsets but would make it
//   hard to implement `get`.
macro_rules! define_string_table {
    ($name:ident, $id:ident, $section:ident, $offset:ident, $offsets:ident, $docs:expr) => {
        #[doc=$docs]
        #[derive(Debug, Default)]
        pub struct $name {
            base_id: BaseId,
            strings: IndexSet<Vec<u8>>,
        }

        impl $name {
            /// Add a string to the string table and return its id.
            ///
            /// If the string already exists, then return the id of the existing string.
            ///
            /// # Panics
            ///
            /// Panics if `bytes` contains a null byte.
            pub fn add<T>(&mut self, bytes: T) -> $id
            where
                T: Into<Vec<u8>>,
            {
                let bytes = bytes.into();
                assert!(!bytes.contains(&0));
                let (index, _) = self.strings.insert_full(bytes);
                $id::new(self.base_id, index)
            }

            /// Return the number of strings in the table.
            #[inline]
            pub fn count(&self) -> usize {
                self.strings.len()
            }

            /// Get a reference to a string in the table.
            ///
            /// # Panics
            ///
            /// Panics if `id` is invalid.
            pub fn get(&self, id: $id) -> &[u8] {
                debug_assert_eq!(self.base_id, id.base_id);
                self.strings.get_index(id.index).map(Vec::as_slice).unwrap()
            }

            /// Write the string table to the `.debug_str` section.
            ///
            /// Returns the offsets at which the strings are written.
            pub fn write<W: Writer>(&self, w: &mut $section<W>) -> Result<$offsets> {
                let mut offsets = Vec::new();
                let mut empty = None;
                for bytes in self.strings.iter() {
                    offsets.push(w.offset());
                    w.write(bytes)?;
                    if empty.is_none() {
                        empty = Some(w.offset());
                    }
                    w.write_u8(0)?;
                }
                // Record the offset of the first null, for use as an empty string.
                if let Some(empty) = empty {
                    offsets.push(empty);
                }

                Ok($offsets {
                    base_id: self.base_id,
                    offsets,
                })
            }
        }

        impl $offsets {
            pub(crate) fn get_empty(&self) -> Option<$id> {
                if self.offsets.is_empty() {
                    None
                } else {
                    // The last offset is always the empty string.
                    Some($id::new(self.base_id, self.offsets.len() - 1))
                }
            }
        }
    };
}

define_id!(StringId, "An identifier for a string in a `StringTable`.");

define_string_table!(
    StringTable,
    StringId,
    DebugStr,
    DebugStrOffset,
    DebugStrOffsets,
    "A table of strings that will be stored in a `.debug_str` section."
);

define_section!(DebugStr, DebugStrOffset, "A writable `.debug_str` section.");

define_offsets!(
    DebugStrOffsets: StringId => DebugStrOffset,
    "The section offsets of all strings within a `.debug_str` section."
);

define_id!(
    LineStringId,
    "An identifier for a string in a `LineStringTable`."
);

define_string_table!(
    LineStringTable,
    LineStringId,
    DebugLineStr,
    DebugLineStrOffset,
    DebugLineStrOffsets,
    "A table of strings that will be stored in a `.debug_line_str` section."
);

define_section!(
    DebugLineStr,
    DebugLineStrOffset,
    "A writable `.debug_line_str` section."
);

define_offsets!(
    DebugLineStrOffsets: LineStringId => DebugLineStrOffset,
    "The section offsets of all strings within a `.debug_line_str` section."
);

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/gimli/unit/src/write/str/tests_150.rs"));
