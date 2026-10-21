#[cfg(test)]
mod tests {
    use super::*;
    use crate::read::EndianSlice;
    use crate::{Endianity, LittleEndian};

    /// Ensure that `Dwarf<R>` is covariant wrt R.
    #[test]
    fn test_dwarf_variance() {
        /// This only needs to compile.
        fn _f<'a: 'b, 'b, E: Endianity>(x: Dwarf<EndianSlice<'a, E>>) -> Dwarf<EndianSlice<'b, E>> {
            x
        }
    }

    /// Ensure that `Unit<R>` is covariant wrt R.
    #[test]
    fn test_dwarf_unit_variance() {
        /// This only needs to compile.
        fn _f<'a: 'b, 'b, E: Endianity>(x: Unit<EndianSlice<'a, E>>) -> Unit<EndianSlice<'b, E>> {
            x
        }
    }

    #[test]
    fn test_send() {
        fn assert_is_send<T: Send>() {}
        assert_is_send::<Dwarf<EndianSlice<'_, LittleEndian>>>();
        assert_is_send::<Unit<EndianSlice<'_, LittleEndian>>>();
    }

    #[test]
    fn test_format_error() {
        let dwarf_sections = DwarfSections::load(|_| -> Result<_> { Ok(vec![1, 2]) }).unwrap();
        let sup_sections = DwarfSections::load(|_| -> Result<_> { Ok(vec![1, 2]) }).unwrap();
        let dwarf = dwarf_sections.borrow_with_sup(&sup_sections, |section| {
            EndianSlice::new(section, LittleEndian)
        });

        match dwarf.debug_str.get_str(DebugStrOffset(1)) {
            Ok(r) => panic!("Unexpected str {:?}", r),
            Err(e) => {
                assert_eq!(
                    dwarf.format_error(e),
                    "Hit the end of input before it was expected at .debug_str+0x1"
                );
            }
        }
        match dwarf.sup().unwrap().debug_str.get_str(DebugStrOffset(1)) {
            Ok(r) => panic!("Unexpected str {:?}", r),
            Err(e) => {
                assert_eq!(
                    dwarf.format_error(e),
                    "Hit the end of input before it was expected at .debug_str(sup)+0x1"
                );
            }
        }
        assert_eq!(dwarf.format_error(Error::Io), Error::Io.description());
    }
}
