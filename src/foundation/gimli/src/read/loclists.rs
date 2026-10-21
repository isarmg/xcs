use crate::common::{
    DebugAddrBase, DebugAddrIndex, DebugLocListsBase, DebugLocListsIndex, DwarfFileType, Encoding,
    LocationListsOffset, SectionId,
};
use crate::constants;
use crate::endianity::Endianity;
use crate::read::{
    lists::ListsHeader, DebugAddr, EndianSlice, Error, Expression, Range, RawRange, Reader,
    ReaderAddress, ReaderOffset, ReaderOffsetId, Result, Section,
};

/// The raw contents of the `.debug_loc` section.
#[derive(Debug, Default, Clone, Copy)]
pub struct DebugLoc<R> {
    pub(crate) section: R,
}

impl<'input, Endian> DebugLoc<EndianSlice<'input, Endian>>
where
    Endian: Endianity,
{
    /// Construct a new `DebugLoc` instance from the data in the `.debug_loc`
    /// section.
    ///
    /// It is the caller's responsibility to read the `.debug_loc` section and
    /// present it as a `&[u8]` slice. That means using some ELF loader on
    /// Linux, a Mach-O loader on macOS, etc.
    ///
    /// ```
    /// use gimli::{DebugLoc, LittleEndian};
    ///
    /// # let buf = [0x00, 0x01, 0x02, 0x03];
    /// # let read_debug_loc_section_somehow = || &buf;
    /// let debug_loc = DebugLoc::new(read_debug_loc_section_somehow(), LittleEndian);
    /// ```
    pub fn new(section: &'input [u8], endian: Endian) -> Self {
        Self::from(EndianSlice::new(section, endian))
    }
}

impl<T> DebugLoc<T> {
    /// Create a `DebugLoc` section that references the data in `self`.
    ///
    /// This is useful when `R` implements `Reader` but `T` does not.
    ///
    /// Used by `DwarfSections::borrow`.
    pub(crate) fn borrow<'a, F, R>(&'a self, mut borrow: F) -> DebugLoc<R>
    where
        F: FnMut(&'a T) -> R,
    {
        borrow(&self.section).into()
    }
}

impl<R> Section<R> for DebugLoc<R> {
    fn id() -> SectionId {
        SectionId::DebugLoc
    }

    fn reader(&self) -> &R {
        &self.section
    }
}

impl<R> From<R> for DebugLoc<R> {
    fn from(section: R) -> Self {
        DebugLoc { section }
    }
}

/// The `DebugLocLists` struct represents the DWARF data
/// found in the `.debug_loclists` section.
#[derive(Debug, Default, Clone, Copy)]
pub struct DebugLocLists<R> {
    section: R,
}

impl<'input, Endian> DebugLocLists<EndianSlice<'input, Endian>>
where
    Endian: Endianity,
{
    /// Construct a new `DebugLocLists` instance from the data in the `.debug_loclists`
    /// section.
    ///
    /// It is the caller's responsibility to read the `.debug_loclists` section and
    /// present it as a `&[u8]` slice. That means using some ELF loader on
    /// Linux, a Mach-O loader on macOS, etc.
    ///
    /// ```
    /// use gimli::{DebugLocLists, LittleEndian};
    ///
    /// # let buf = [0x00, 0x01, 0x02, 0x03];
    /// # let read_debug_loclists_section_somehow = || &buf;
    /// let debug_loclists = DebugLocLists::new(read_debug_loclists_section_somehow(), LittleEndian);
    /// ```
    pub fn new(section: &'input [u8], endian: Endian) -> Self {
        Self::from(EndianSlice::new(section, endian))
    }
}

impl<T> DebugLocLists<T> {
    /// Create a `DebugLocLists` section that references the data in `self`.
    ///
    /// This is useful when `R` implements `Reader` but `T` does not.
    ///
    /// Used by `DwarfSections::borrow`.
    pub(crate) fn borrow<'a, F, R>(&'a self, mut borrow: F) -> DebugLocLists<R>
    where
        F: FnMut(&'a T) -> R,
    {
        borrow(&self.section).into()
    }
}

impl<R> Section<R> for DebugLocLists<R> {
    fn id() -> SectionId {
        SectionId::DebugLocLists
    }

    fn reader(&self) -> &R {
        &self.section
    }
}

impl<R> From<R> for DebugLocLists<R> {
    fn from(section: R) -> Self {
        DebugLocLists { section }
    }
}

pub(crate) type LocListsHeader = ListsHeader;

impl<Offset> DebugLocListsBase<Offset>
where
    Offset: ReaderOffset,
{
    /// Returns a `DebugLocListsBase` with the default value of DW_AT_loclists_base
    /// for the given `Encoding` and `DwarfFileType`.
    pub fn default_for_encoding_and_file(
        encoding: Encoding,
        file_type: DwarfFileType,
    ) -> DebugLocListsBase<Offset> {
        if encoding.version >= 5 && file_type == DwarfFileType::Dwo {
            // In .dwo files, the compiler omits the DW_AT_loclists_base attribute (because there is
            // only a single unit in the file) but we must skip past the header, which the attribute
            // would normally do for us.
            DebugLocListsBase(Offset::from_u8(LocListsHeader::size_for_encoding(encoding)))
        } else {
            DebugLocListsBase(Offset::from_u8(0))
        }
    }
}

/// The DWARF data found in `.debug_loc` and `.debug_loclists` sections.
#[derive(Debug, Default, Clone, Copy)]
pub struct LocationLists<R> {
    debug_loc: DebugLoc<R>,
    debug_loclists: DebugLocLists<R>,
}

impl<R> LocationLists<R> {
    /// Construct a new `LocationLists` instance from the data in the `.debug_loc` and
    /// `.debug_loclists` sections.
    pub fn new(debug_loc: DebugLoc<R>, debug_loclists: DebugLocLists<R>) -> LocationLists<R> {
        LocationLists {
            debug_loc,
            debug_loclists,
        }
    }
}

impl<T> LocationLists<T> {
    /// Create a `LocationLists` that references the data in `self`.
    ///
    /// This is useful when `R` implements `Reader` but `T` does not.
    ///
    /// Used by `Dwarf::borrow`.
    pub fn borrow<'a, F, R>(&'a self, mut borrow: F) -> LocationLists<R>
    where
        F: FnMut(&'a T) -> R,
    {
        LocationLists {
            debug_loc: borrow(&self.debug_loc.section).into(),
            debug_loclists: borrow(&self.debug_loclists.section).into(),
        }
    }
}

impl<R: Reader> LocationLists<R> {
    /// Iterate over the `LocationListEntry`s starting at the given offset.
    ///
    /// The `unit_encoding` must match the compilation unit that the
    /// offset was contained in.
    ///
    /// The `base_address` should be obtained from the `DW_AT_low_pc` attribute in the
    /// `DW_TAG_compile_unit` entry for the compilation unit that contains this location
    /// list.
    ///
    /// Can be [used with
    /// `FallibleIterator`](./index.html#using-with-fallibleiterator).
    pub fn locations(
        &self,
        offset: LocationListsOffset<R::Offset>,
        unit_encoding: Encoding,
        base_address: u64,
        debug_addr: &DebugAddr<R>,
        debug_addr_base: DebugAddrBase<R::Offset>,
    ) -> Result<LocListIter<R>> {
        Ok(LocListIter::new(
            self.raw_locations(offset, unit_encoding)?,
            base_address,
            debug_addr.clone(),
            debug_addr_base,
        ))
    }

    /// Similar to `locations`, but with special handling for .dwo files.
    /// This should only been used when this `LocationLists` was loaded from a
    /// .dwo file.
    pub fn locations_dwo(
        &self,
        offset: LocationListsOffset<R::Offset>,
        unit_encoding: Encoding,
        base_address: u64,
        debug_addr: &DebugAddr<R>,
        debug_addr_base: DebugAddrBase<R::Offset>,
    ) -> Result<LocListIter<R>> {
        Ok(LocListIter::new(
            self.raw_locations_dwo(offset, unit_encoding)?,
            base_address,
            debug_addr.clone(),
            debug_addr_base,
        ))
    }

    /// Iterate over the raw `LocationListEntry`s starting at the given offset.
    ///
    /// The `unit_encoding` must match the compilation unit that the
    /// offset was contained in.
    ///
    /// This iterator does not perform any processing of the location entries,
    /// such as handling base addresses.
    ///
    /// Can be [used with
    /// `FallibleIterator`](./index.html#using-with-fallibleiterator).
    pub fn raw_locations(
        &self,
        offset: LocationListsOffset<R::Offset>,
        unit_encoding: Encoding,
    ) -> Result<RawLocListIter<R>> {
        let (mut input, format) = if unit_encoding.version <= 4 {
            (self.debug_loc.section.clone(), LocListsFormat::Bare)
        } else {
            (self.debug_loclists.section.clone(), LocListsFormat::Lle)
        };
        input.skip(offset.0)?;
        Ok(RawLocListIter::new(input, unit_encoding, format))
    }

    /// Similar to `raw_locations`, but with special handling for .dwo files.
    /// This should only been used when this `LocationLists` was loaded from a
    /// .dwo file.
    pub fn raw_locations_dwo(
        &self,
        offset: LocationListsOffset<R::Offset>,
        unit_encoding: Encoding,
    ) -> Result<RawLocListIter<R>> {
        let mut input = if unit_encoding.version <= 4 {
            // In the GNU split dwarf extension the locations are present in the
            // .debug_loc section but are encoded with the DW_LLE values used
            // for the DWARF 5 .debug_loclists section.
            self.debug_loc.section.clone()
        } else {
            self.debug_loclists.section.clone()
        };
        input.skip(offset.0)?;
        Ok(RawLocListIter::new(
            input,
            unit_encoding,
            LocListsFormat::Lle,
        ))
    }

    /// Returns the `.debug_loclists` offset at the given `base` and `index`.
    ///
    /// The `base` must be the `DW_AT_loclists_base` value from the compilation unit DIE.
    /// This is an offset that points to the first entry following the header.
    ///
    /// The `index` is the value of a `DW_FORM_loclistx` attribute.
    pub fn get_offset(
        &self,
        unit_encoding: Encoding,
        base: DebugLocListsBase<R::Offset>,
        index: DebugLocListsIndex<R::Offset>,
    ) -> Result<LocationListsOffset<R::Offset>> {
        let format = unit_encoding.format;
        let input = &mut self.debug_loclists.section.clone();
        input.skip(base.0)?;
        input.skip(R::Offset::from_u64(
            index.0.into_u64() * u64::from(format.word_size()),
        )?)?;
        input
            .read_offset(format)
            .map(|x| LocationListsOffset(base.0 + x))
    }

    /// Call `Reader::lookup_offset_id` for each section, and return the first match.
    pub fn lookup_offset_id(&self, id: ReaderOffsetId) -> Option<(SectionId, R::Offset)> {
        self.debug_loc
            .lookup_offset_id(id)
            .or_else(|| self.debug_loclists.lookup_offset_id(id))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocListsFormat {
    /// The bare location list format used before DWARF 5.
    Bare,
    /// The DW_LLE encoded range list format used in DWARF 5 and the non-standard GNU
    /// split dwarf extension.
    Lle,
}

/// A raw iterator over a location list.
///
/// This iterator does not perform any processing of the location entries,
/// such as handling base addresses.
#[derive(Debug)]
pub struct RawLocListIter<R: Reader> {
    input: R,
    encoding: Encoding,
    format: LocListsFormat,
}

/// A raw entry in .debug_loclists.
#[derive(Clone, Debug)]
pub enum RawLocListEntry<R: Reader> {
    /// A location from DWARF version <= 4.
    AddressOrOffsetPair {
        /// Start of range. May be an address or an offset.
        begin: u64,
        /// End of range. May be an address or an offset.
        end: u64,
        /// expression
        data: Expression<R>,
    },
    /// DW_LLE_base_address
    BaseAddress {
        /// base address
        addr: u64,
    },
    /// DW_LLE_base_addressx
    BaseAddressx {
        /// base address
        addr: DebugAddrIndex<R::Offset>,
    },
    /// DW_LLE_startx_endx
    StartxEndx {
        /// start of range
        begin: DebugAddrIndex<R::Offset>,
        /// end of range
        end: DebugAddrIndex<R::Offset>,
        /// expression
        data: Expression<R>,
    },
    /// DW_LLE_startx_length
    StartxLength {
        /// start of range
        begin: DebugAddrIndex<R::Offset>,
        /// length of range
        length: u64,
        /// expression
        data: Expression<R>,
    },
    /// DW_LLE_offset_pair
    OffsetPair {
        /// start of range
        begin: u64,
        /// end of range
        end: u64,
        /// expression
        data: Expression<R>,
    },
    /// DW_LLE_default_location
    DefaultLocation {
        /// expression
        data: Expression<R>,
    },
    /// DW_LLE_start_end
    StartEnd {
        /// start of range
        begin: u64,
        /// end of range
        end: u64,
        /// expression
        data: Expression<R>,
    },
    /// DW_LLE_start_length
    StartLength {
        /// start of range
        begin: u64,
        /// length of range
        length: u64,
        /// expression
        data: Expression<R>,
    },
}

fn parse_data<R: Reader>(input: &mut R, encoding: Encoding) -> Result<Expression<R>> {
    if encoding.version >= 5 {
        let len = R::Offset::from_u64(input.read_uleb128()?)?;
        Ok(Expression(input.split(len)?))
    } else {
        // In the GNU split-dwarf extension this is a fixed 2 byte value.
        let len = R::Offset::from_u16(input.read_u16()?);
        Ok(Expression(input.split(len)?))
    }
}

impl<R: Reader> RawLocListEntry<R> {
    /// Parse a location list entry from `.debug_loclists`
    fn parse(input: &mut R, encoding: Encoding, format: LocListsFormat) -> Result<Option<Self>> {
        Ok(match format {
            LocListsFormat::Bare => {
                let range = RawRange::parse(input, encoding.address_size)?;
                if range.is_end() {
                    None
                } else if range.is_base_address(encoding.address_size) {
                    Some(RawLocListEntry::BaseAddress { addr: range.end })
                } else {
                    let len = R::Offset::from_u16(input.read_u16()?);
                    let data = Expression(input.split(len)?);
                    Some(RawLocListEntry::AddressOrOffsetPair {
                        begin: range.begin,
                        end: range.end,
                        data,
                    })
                }
            }
            LocListsFormat::Lle => match constants::DwLle(input.read_u8()?) {
                constants::DW_LLE_end_of_list => None,
                constants::DW_LLE_base_addressx => Some(RawLocListEntry::BaseAddressx {
                    addr: DebugAddrIndex(input.read_uleb128().and_then(R::Offset::from_u64)?),
                }),
                constants::DW_LLE_startx_endx => Some(RawLocListEntry::StartxEndx {
                    begin: DebugAddrIndex(input.read_uleb128().and_then(R::Offset::from_u64)?),
                    end: DebugAddrIndex(input.read_uleb128().and_then(R::Offset::from_u64)?),
                    data: parse_data(input, encoding)?,
                }),
                constants::DW_LLE_startx_length => Some(RawLocListEntry::StartxLength {
                    begin: DebugAddrIndex(input.read_uleb128().and_then(R::Offset::from_u64)?),
                    length: if encoding.version >= 5 {
                        input.read_uleb128()?
                    } else {
                        // In the GNU split-dwarf extension this is a fixed 4 byte value.
                        input.read_u32()? as u64
                    },
                    data: parse_data(input, encoding)?,
                }),
                constants::DW_LLE_offset_pair => Some(RawLocListEntry::OffsetPair {
                    begin: input.read_uleb128()?,
                    end: input.read_uleb128()?,
                    data: parse_data(input, encoding)?,
                }),
                constants::DW_LLE_default_location => Some(RawLocListEntry::DefaultLocation {
                    data: parse_data(input, encoding)?,
                }),
                constants::DW_LLE_base_address => Some(RawLocListEntry::BaseAddress {
                    addr: input.read_address(encoding.address_size)?,
                }),
                constants::DW_LLE_start_end => Some(RawLocListEntry::StartEnd {
                    begin: input.read_address(encoding.address_size)?,
                    end: input.read_address(encoding.address_size)?,
                    data: parse_data(input, encoding)?,
                }),
                constants::DW_LLE_start_length => Some(RawLocListEntry::StartLength {
                    begin: input.read_address(encoding.address_size)?,
                    length: input.read_uleb128()?,
                    data: parse_data(input, encoding)?,
                }),
                entry => {
                    return Err(Error::UnknownLocListsEntry(entry));
                }
            },
        })
    }
}

impl<R: Reader> RawLocListIter<R> {
    /// Construct a `RawLocListIter`.
    fn new(input: R, encoding: Encoding, format: LocListsFormat) -> RawLocListIter<R> {
        RawLocListIter {
            input,
            encoding,
            format,
        }
    }

    /// Advance the iterator to the next location.
    pub fn next(&mut self) -> Result<Option<RawLocListEntry<R>>> {
        if self.input.is_empty() {
            return Ok(None);
        }

        match RawLocListEntry::parse(&mut self.input, self.encoding, self.format) {
            Ok(entry) => {
                if entry.is_none() {
                    self.input.empty();
                }
                Ok(entry)
            }
            Err(e) => {
                self.input.empty();
                Err(e)
            }
        }
    }
}

#[cfg(feature = "fallible-iterator")]
impl<R: Reader> fallible_iterator::FallibleIterator for RawLocListIter<R> {
    type Item = RawLocListEntry<R>;
    type Error = Error;

    fn next(&mut self) -> ::core::result::Result<Option<Self::Item>, Self::Error> {
        RawLocListIter::next(self)
    }
}

/// An iterator over a location list.
///
/// This iterator internally handles processing of base address selection entries
/// and list end entries.  Thus, it only returns location entries that are valid
/// and already adjusted for the base address.
#[derive(Debug)]
pub struct LocListIter<R: Reader> {
    raw: RawLocListIter<R>,
    base_address: u64,
    debug_addr: DebugAddr<R>,
    debug_addr_base: DebugAddrBase<R::Offset>,
}

impl<R: Reader> LocListIter<R> {
    /// Construct a `LocListIter`.
    fn new(
        raw: RawLocListIter<R>,
        base_address: u64,
        debug_addr: DebugAddr<R>,
        debug_addr_base: DebugAddrBase<R::Offset>,
    ) -> LocListIter<R> {
        LocListIter {
            raw,
            base_address,
            debug_addr,
            debug_addr_base,
        }
    }

    #[inline]
    fn get_address(&self, index: DebugAddrIndex<R::Offset>) -> Result<u64> {
        self.debug_addr
            .get_address(self.raw.encoding.address_size, self.debug_addr_base, index)
    }

    /// Advance the iterator to the next location.
    pub fn next(&mut self) -> Result<Option<LocationListEntry<R>>> {
        loop {
            let raw_loc = match self.raw.next()? {
                Some(loc) => loc,
                None => return Ok(None),
            };

            let loc = self.convert_raw(raw_loc)?;
            if loc.is_some() {
                return Ok(loc);
            }
        }
    }

    /// Return the next raw location.
    ///
    /// The raw location should be passed to `convert_raw`.
    #[doc(hidden)]
    pub fn next_raw(&mut self) -> Result<Option<RawLocListEntry<R>>> {
        self.raw.next()
    }

    /// Convert a raw location into a location, and update the state of the iterator.
    ///
    /// The raw location should have been obtained from `next_raw`.
    #[doc(hidden)]
    pub fn convert_raw(
        &mut self,
        raw_loc: RawLocListEntry<R>,
    ) -> Result<Option<LocationListEntry<R>>> {
        let address_size = self.raw.encoding.address_size;

        let (range, data) = match raw_loc {
            RawLocListEntry::BaseAddress { addr } => {
                self.base_address = addr;
                return Ok(None);
            }
            RawLocListEntry::BaseAddressx { addr } => {
                self.base_address = self.get_address(addr)?;
                return Ok(None);
            }
            RawLocListEntry::StartxEndx { begin, end, data } => {
                let begin = self.get_address(begin)?;
                let end = self.get_address(end)?;
                (Range { begin, end }, data)
            }
            RawLocListEntry::StartxLength {
                begin,
                length,
                data,
            } => {
                let begin = self.get_address(begin)?;
                let end = begin.wrapping_add_sized(length, address_size);
                (Range { begin, end }, data)
            }
            RawLocListEntry::DefaultLocation { data } => (
                Range {
                    begin: 0,
                    end: u64::MAX,
                },
                data,
            ),
            RawLocListEntry::AddressOrOffsetPair { begin, end, data }
            | RawLocListEntry::OffsetPair { begin, end, data } => {
                // Skip tombstone entries (see below).
                if self.base_address >= u64::min_tombstone(address_size) {
                    return Ok(None);
                }
                let mut range = Range { begin, end };
                range.add_base_address(self.base_address, address_size);
                (range, data)
            }
            RawLocListEntry::StartEnd { begin, end, data } => (Range { begin, end }, data),
            RawLocListEntry::StartLength {
                begin,
                length,
                data,
            } => {
                let end = begin.wrapping_add_sized(length, address_size);
                (Range { begin, end }, data)
            }
        };

        // Skip tombstone entries.
        //
        // DWARF specifies a tombstone value of -1 or -2, but many linkers use 0 or 1.
        // However, 0/1 may be a valid address, so we can't always reliably skip them.
        // One case where we can skip them is for address pairs, where both values are
        // replaced by tombstones and thus `begin` equals `end`. Since these entries
        // are empty, it's safe to skip them even if they aren't tombstones.
        //
        // In addition to skipping tombstone entries, we also skip invalid entries
        // where `begin` is greater than `end`. This can occur due to compiler bugs.
        if range.begin >= u64::min_tombstone(address_size) || range.begin >= range.end {
            return Ok(None);
        }

        Ok(Some(LocationListEntry { range, data }))
    }
}

#[cfg(feature = "fallible-iterator")]
impl<R: Reader> fallible_iterator::FallibleIterator for LocListIter<R> {
    type Item = LocationListEntry<R>;
    type Error = Error;

    fn next(&mut self) -> ::core::result::Result<Option<Self::Item>, Self::Error> {
        LocListIter::next(self)
    }
}

/// A location list entry from the `.debug_loc` or `.debug_loclists` sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocationListEntry<R: Reader> {
    /// The address range that this location is valid for.
    pub range: Range,

    /// The data containing a single location description.
    pub data: Expression<R>,
}

#[cfg(test)]
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../test/foundation/gimli/unit/src/read/loclists/tests_687.rs"));
