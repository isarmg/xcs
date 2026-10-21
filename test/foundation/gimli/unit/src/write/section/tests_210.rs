#[cfg(test)]
#[cfg(feature = "read")]
mod tests {
    use super::*;
    use crate::{read, write::EndianVec, Endianity};

    impl<E: Endianity> Sections<EndianVec<E>> {
        pub(crate) fn read(&self, endian: E) -> read::Dwarf<read::EndianSlice<'_, E>> {
            read::Dwarf::load(|section_id| -> read::Result<_> {
                Ok(read::EndianSlice::new(
                    self.get(section_id).map(|w| w.slice()).unwrap_or_default(),
                    endian,
                ))
            })
            .unwrap()
        }
    }
}
