use crate::{Encode, Section, SectionId, encoding_size};
use alloc::borrow::Cow;
use alloc::vec::Vec;

/// A custom section holding arbitrary data.
#[derive(Clone, Debug, PartialEq)]
pub struct CustomSection<'a> {
    /// The name of this custom section.
    pub name: Cow<'a, str>,
    /// This custom section's data.
    pub data: Cow<'a, [u8]>,
}

impl Encode for CustomSection<'_> {
    fn encode(&self, sink: &mut Vec<u8>) {
        let encoded_name_len = encoding_size(u32::try_from(self.name.len()).unwrap());
        (encoded_name_len + self.name.len() + self.data.len()).encode(sink);
        self.name.encode(sink);
        sink.extend(&*self.data);
    }
}

impl Section for CustomSection<'_> {
    fn id(&self) -> u8 {
        SectionId::Custom.into()
    }
}

/// A raw custom section where the bytes specified contain the leb-encoded
/// length of the custom section, the custom section's name, and the custom
/// section's data.
#[derive(Clone, Debug)]
pub struct RawCustomSection<'a>(pub &'a [u8]);

impl Encode for RawCustomSection<'_> {
    fn encode(&self, sink: &mut Vec<u8>) {
        sink.extend(self.0);
    }
}

impl Section for RawCustomSection<'_> {
    fn id(&self) -> u8 {
        SectionId::Custom.into()
    }
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../../test/foundation/wasm-encoder/unit/src/core/custom/tests_47.rs"));
