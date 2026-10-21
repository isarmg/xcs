use crate::common::{Format, SectionId};
use crate::constants;
use crate::endianity::Endianity;
use crate::leb128;
use crate::write::{Address, Error, Result};

/// A trait for writing the data to a DWARF section.
///
/// All write operations append to the section unless otherwise specified.
#[allow(clippy::len_without_is_empty)]
pub trait Writer {
    /// The endianity of bytes that are written.
    type Endian: Endianity;

    /// Return the endianity of bytes that are written.
    fn endian(&self) -> Self::Endian;

    /// Return the current section length.
    ///
    /// This may be used as an offset for future `write_at` calls.
    fn len(&self) -> usize;

    /// Write a slice.
    fn write(&mut self, bytes: &[u8]) -> Result<()>;

    /// Write a slice at a given offset.
    ///
    /// The write must not extend past the current section length.
    fn write_at(&mut self, offset: usize, bytes: &[u8]) -> Result<()>;

    /// Write an address.
    ///
    /// If the writer supports relocations, then it must provide its own implementation
    /// of this method.
    // TODO: use write_reference instead?
    fn write_address(&mut self, address: Address, size: u8) -> Result<()> {
        match address {
            Address::Constant(val) => self.write_udata(val, size),
            Address::Symbol { .. } => Err(Error::InvalidAddress),
        }
    }

    /// Write an address with a `.eh_frame` pointer encoding.
    ///
    /// The given size is only used for `DW_EH_PE_absptr` formats.
    ///
    /// If the writer supports relocations, then it must provide its own implementation
    /// of this method.
    fn write_eh_pointer(
        &mut self,
        address: Address,
        eh_pe: constants::DwEhPe,
        size: u8,
    ) -> Result<()> {
        match address {
            Address::Constant(val) => {
                // Indirect doesn't matter here.
                let val = match eh_pe.application() {
                    constants::DW_EH_PE_absptr => val,
                    constants::DW_EH_PE_pcrel => {
                        // TODO: better handling of sign
                        let offset = self.len() as u64;
                        val.wrapping_sub(offset)
                    }
                    _ => {
                        return Err(Error::UnsupportedPointerEncoding(eh_pe));
                    }
                };
                self.write_eh_pointer_data(val, eh_pe.format(), size)
            }
            Address::Symbol { .. } => Err(Error::InvalidAddress),
        }
    }

    /// Write a value with a `.eh_frame` pointer format.
    ///
    /// The given size is only used for `DW_EH_PE_absptr` formats.
    ///
    /// This must not be used directly for values that may require relocation.
    fn write_eh_pointer_data(
        &mut self,
        val: u64,
        format: constants::DwEhPe,
        size: u8,
    ) -> Result<()> {
        match format {
            constants::DW_EH_PE_absptr => self.write_udata(val, size),
            constants::DW_EH_PE_uleb128 => self.write_uleb128(val),
            constants::DW_EH_PE_udata2 => self.write_udata(val, 2),
            constants::DW_EH_PE_udata4 => self.write_udata(val, 4),
            constants::DW_EH_PE_udata8 => self.write_udata(val, 8),
            constants::DW_EH_PE_sleb128 => self.write_sleb128(val as i64),
            constants::DW_EH_PE_sdata2 => self.write_sdata(val as i64, 2),
            constants::DW_EH_PE_sdata4 => self.write_sdata(val as i64, 4),
            constants::DW_EH_PE_sdata8 => self.write_sdata(val as i64, 8),
            _ => Err(Error::UnsupportedPointerEncoding(format)),
        }
    }

    /// Write an offset that is relative to the start of the given section.
    ///
    /// If the writer supports relocations, then it must provide its own implementation
    /// of this method.
    fn write_offset(&mut self, val: usize, _section: SectionId, size: u8) -> Result<()> {
        self.write_udata(val as u64, size)
    }

    /// Write an offset that is relative to the start of the given section.
    ///
    /// If the writer supports relocations, then it must provide its own implementation
    /// of this method.
    fn write_offset_at(
        &mut self,
        offset: usize,
        val: usize,
        _section: SectionId,
        size: u8,
    ) -> Result<()> {
        self.write_udata_at(offset, val as u64, size)
    }

    /// Write a reference to a symbol.
    ///
    /// If the writer supports symbols, then it must provide its own implementation
    /// of this method.
    fn write_reference(&mut self, _symbol: usize, _size: u8) -> Result<()> {
        Err(Error::InvalidReference)
    }

    /// Write a u8.
    fn write_u8(&mut self, val: u8) -> Result<()> {
        let bytes = [val];
        self.write(&bytes)
    }

    /// Write a u16.
    fn write_u16(&mut self, val: u16) -> Result<()> {
        let mut bytes = [0; 2];
        self.endian().write_u16(&mut bytes, val);
        self.write(&bytes)
    }

    /// Write a u32.
    fn write_u32(&mut self, val: u32) -> Result<()> {
        let mut bytes = [0; 4];
        self.endian().write_u32(&mut bytes, val);
        self.write(&bytes)
    }

    /// Write a u64.
    fn write_u64(&mut self, val: u64) -> Result<()> {
        let mut bytes = [0; 8];
        self.endian().write_u64(&mut bytes, val);
        self.write(&bytes)
    }

    /// Write a u8 at the given offset.
    fn write_u8_at(&mut self, offset: usize, val: u8) -> Result<()> {
        let bytes = [val];
        self.write_at(offset, &bytes)
    }

    /// Write a u16 at the given offset.
    fn write_u16_at(&mut self, offset: usize, val: u16) -> Result<()> {
        let mut bytes = [0; 2];
        self.endian().write_u16(&mut bytes, val);
        self.write_at(offset, &bytes)
    }

    /// Write a u32 at the given offset.
    fn write_u32_at(&mut self, offset: usize, val: u32) -> Result<()> {
        let mut bytes = [0; 4];
        self.endian().write_u32(&mut bytes, val);
        self.write_at(offset, &bytes)
    }

    /// Write a u64 at the given offset.
    fn write_u64_at(&mut self, offset: usize, val: u64) -> Result<()> {
        let mut bytes = [0; 8];
        self.endian().write_u64(&mut bytes, val);
        self.write_at(offset, &bytes)
    }

    /// Write unsigned data of the given size.
    ///
    /// Returns an error if the value is too large for the size.
    /// This must not be used directly for values that may require relocation.
    fn write_udata(&mut self, val: u64, size: u8) -> Result<()> {
        match size {
            1 => {
                let write_val = val as u8;
                if val != u64::from(write_val) {
                    return Err(Error::ValueTooLarge);
                }
                self.write_u8(write_val)
            }
            2 => {
                let write_val = val as u16;
                if val != u64::from(write_val) {
                    return Err(Error::ValueTooLarge);
                }
                self.write_u16(write_val)
            }
            4 => {
                let write_val = val as u32;
                if val != u64::from(write_val) {
                    return Err(Error::ValueTooLarge);
                }
                self.write_u32(write_val)
            }
            8 => self.write_u64(val),
            otherwise => Err(Error::UnsupportedWordSize(otherwise)),
        }
    }

    /// Write signed data of the given size.
    ///
    /// Returns an error if the value is too large for the size.
    /// This must not be used directly for values that may require relocation.
    fn write_sdata(&mut self, val: i64, size: u8) -> Result<()> {
        match size {
            1 => {
                let write_val = val as i8;
                if val != i64::from(write_val) {
                    return Err(Error::ValueTooLarge);
                }
                self.write_u8(write_val as u8)
            }
            2 => {
                let write_val = val as i16;
                if val != i64::from(write_val) {
                    return Err(Error::ValueTooLarge);
                }
                self.write_u16(write_val as u16)
            }
            4 => {
                let write_val = val as i32;
                if val != i64::from(write_val) {
                    return Err(Error::ValueTooLarge);
                }
                self.write_u32(write_val as u32)
            }
            8 => self.write_u64(val as u64),
            otherwise => Err(Error::UnsupportedWordSize(otherwise)),
        }
    }

    /// Write a word of the given size at the given offset.
    ///
    /// Returns an error if the value is too large for the size.
    /// This must not be used directly for values that may require relocation.
    fn write_udata_at(&mut self, offset: usize, val: u64, size: u8) -> Result<()> {
        match size {
            1 => {
                let write_val = val as u8;
                if val != u64::from(write_val) {
                    return Err(Error::ValueTooLarge);
                }
                self.write_u8_at(offset, write_val)
            }
            2 => {
                let write_val = val as u16;
                if val != u64::from(write_val) {
                    return Err(Error::ValueTooLarge);
                }
                self.write_u16_at(offset, write_val)
            }
            4 => {
                let write_val = val as u32;
                if val != u64::from(write_val) {
                    return Err(Error::ValueTooLarge);
                }
                self.write_u32_at(offset, write_val)
            }
            8 => self.write_u64_at(offset, val),
            otherwise => Err(Error::UnsupportedWordSize(otherwise)),
        }
    }

    /// Write an unsigned LEB128 encoded integer.
    fn write_uleb128(&mut self, val: u64) -> Result<()> {
        let mut bytes = [0u8; 10];
        // bytes is long enough so this will never fail.
        let len = leb128::write::unsigned(&mut { &mut bytes[..] }, val).unwrap();
        self.write(&bytes[..len])
    }

    /// Read an unsigned LEB128 encoded integer.
    fn write_sleb128(&mut self, val: i64) -> Result<()> {
        let mut bytes = [0u8; 10];
        // bytes is long enough so this will never fail.
        let len = leb128::write::signed(&mut { &mut bytes[..] }, val).unwrap();
        self.write(&bytes[..len])
    }

    /// Write an initial length according to the given DWARF format.
    ///
    /// This will only write a length of zero, since the length isn't
    /// known yet, and a subsequent call to `write_initial_length_at`
    /// will write the actual length.
    fn write_initial_length(&mut self, format: Format) -> Result<InitialLengthOffset> {
        if format == Format::Dwarf64 {
            self.write_u32(0xffff_ffff)?;
        }
        let offset = InitialLengthOffset(self.len());
        self.write_udata(0, format.word_size())?;
        Ok(offset)
    }

    /// Write an initial length at the given offset according to the given DWARF format.
    ///
    /// `write_initial_length` must have previously returned the offset.
    fn write_initial_length_at(
        &mut self,
        offset: InitialLengthOffset,
        length: u64,
        format: Format,
    ) -> Result<()> {
        self.write_udata_at(offset.0, length, format.word_size())
    }
}

/// The offset at which an initial length should be written.
#[derive(Debug, Clone, Copy)]
pub struct InitialLengthOffset(usize);

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/gimli/unit/src/write/writer/tests_327.rs"));
