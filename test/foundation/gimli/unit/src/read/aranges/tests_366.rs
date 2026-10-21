#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{DebugInfoOffset, Format};
    use crate::endianity::LittleEndian;
    use crate::read::EndianSlice;

    #[test]
    fn test_iterate_headers() {
        #[rustfmt::skip]
        let buf = [
            // 32-bit length = 28.
            0x1c, 0x00, 0x00, 0x00,
            // Version.
            0x02, 0x00,
            // Offset.
            0x01, 0x02, 0x03, 0x04,
            // Address size.
            0x04,
            // Segment size.
            0x00,
            // Dummy padding and arange tuples.
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,

            // 32-bit length = 36.
            0x24, 0x00, 0x00, 0x00,
            // Version.
            0x02, 0x00,
            // Offset.
            0x11, 0x12, 0x13, 0x14,
            // Address size.
            0x04,
            // Segment size.
            0x00,
            // Dummy padding and arange tuples.
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let debug_aranges = DebugAranges::new(&buf, LittleEndian);
        let mut headers = debug_aranges.headers();

        let header = headers
            .next()
            .expect("should parse header ok")
            .expect("should have a header");
        assert_eq!(header.offset(), DebugArangesOffset(0));
        assert_eq!(header.debug_info_offset(), DebugInfoOffset(0x0403_0201));

        let header = headers
            .next()
            .expect("should parse header ok")
            .expect("should have a header");
        assert_eq!(header.offset(), DebugArangesOffset(0x20));
        assert_eq!(header.debug_info_offset(), DebugInfoOffset(0x1413_1211));
    }

    #[test]
    fn test_parse_header_ok() {
        #[rustfmt::skip]
        let buf = [
            // 32-bit length = 28 (8 bytes header, 4 bytes padding, 16 bytes tuple data).
            0x1c, 0x00, 0x00, 0x00,
            // Version.
            0x02, 0x00,
            // Offset.
            0x01, 0x02, 0x03, 0x04,
            // Address size.
            0x08,
            // Segment size.
            0x00,
            // Length to here = 12, tuple length = 16.
            // Padding to tuple length multiple = 4.
            0x10, 0x00, 0x00, 0x00,

            // Dummy arange tuple data.
            0x20, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,

            // Dummy next arange.
            0x30, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];

        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        let header =
            ArangeHeader::parse(rest, DebugArangesOffset(0x10)).expect("should parse header ok");

        assert_eq!(
            *rest,
            EndianSlice::new(&buf[buf.len() - 16..], LittleEndian)
        );
        assert_eq!(
            header,
            ArangeHeader {
                offset: DebugArangesOffset(0x10),
                encoding: Encoding {
                    format: Format::Dwarf32,
                    version: 2,
                    address_size: 8,
                },
                length: 0x1c,
                debug_info_offset: DebugInfoOffset(0x0403_0201),
                entries: EndianSlice::new(&buf[buf.len() - 32..buf.len() - 16], LittleEndian),
            }
        );
    }

    #[test]
    fn test_parse_header_overflow_error() {
        #[rustfmt::skip]
        let buf = [
            // 32-bit length = 32.
            0x20, 0x00, 0x00, 0x00,
            // Version.
            0x02, 0x00,
            // Offset.
            0x01, 0x02, 0x03, 0x04,
            // Address size.
            0xff,
            // Segment size.
            0x00,
            // Length to here = 12, tuple length = 20.
            // Padding to tuple length multiple = 4.
            0x10, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,

            // Dummy arange tuple data.
            0x20, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,

            // Dummy next arange.
            0x30, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];

        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        let error = ArangeHeader::parse(rest, DebugArangesOffset(0x10))
            .expect_err("should fail to parse header");
        assert_eq!(error, Error::UnsupportedAddressSize(0xff));
    }

    #[test]
    fn test_parse_header_div_by_zero_error() {
        #[rustfmt::skip]
        let buf = [
            // 32-bit length = 32.
            0x20, 0x00, 0x00, 0x00,
            // Version.
            0x02, 0x00,
            // Offset.
            0x01, 0x02, 0x03, 0x04,
            // Address size = 0. Could cause a division by zero if we aren't
            // careful.
            0x00,
            // Segment size.
            0x00,
            // Length to here = 12, tuple length = 20.
            // Padding to tuple length multiple = 4.
            0x10, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,

            // Dummy arange tuple data.
            0x20, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,

            // Dummy next arange.
            0x30, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];

        let rest = &mut EndianSlice::new(&buf, LittleEndian);

        let error = ArangeHeader::parse(rest, DebugArangesOffset(0x10))
            .expect_err("should fail to parse header");
        assert_eq!(error, Error::UnsupportedAddressSize(0));
    }

    #[test]
    fn test_parse_entry_ok() {
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 2,
            address_size: 4,
        };
        let buf = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09];
        let mut iter = ArangeEntryIter {
            input: EndianSlice::new(&buf, LittleEndian),
            encoding,
        };
        let entry = iter.next().expect("should parse entry ok");
        assert_eq!(
            iter.input,
            EndianSlice::new(&buf[buf.len() - 1..], LittleEndian)
        );
        assert_eq!(
            entry,
            Some(ArangeEntry {
                range: Range {
                    begin: 0x0403_0201,
                    end: 0x0403_0201 + 0x0807_0605,
                },
                length: 0x0807_0605,
            })
        );
    }

    #[test]
    fn test_parse_entry_zero() {
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 2,
            address_size: 4,
        };
        #[rustfmt::skip]
        let buf = [
            // Zero tuple.
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            // Address.
            0x01, 0x02, 0x03, 0x04,
            // Length.
            0x05, 0x06, 0x07, 0x08,
            // Next tuple.
            0x09
        ];
        let mut iter = ArangeEntryIter {
            input: EndianSlice::new(&buf, LittleEndian),
            encoding,
        };
        let entry = iter.next().expect("should parse entry ok");
        assert_eq!(
            iter.input,
            EndianSlice::new(&buf[buf.len() - 1..], LittleEndian)
        );
        assert_eq!(
            entry,
            Some(ArangeEntry {
                range: Range {
                    begin: 0x0403_0201,
                    end: 0x0403_0201 + 0x0807_0605,
                },
                length: 0x0807_0605,
            })
        );
    }

    #[test]
    fn test_parse_entry_overflow_32() {
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 2,
            address_size: 4,
        };
        #[rustfmt::skip]
        let buf = [
            // Address.
            0x01, 0x02, 0x03, 0x84,
            // Length.
            0x05, 0x06, 0x07, 0x88,
            // Next tuple.
            0x09
        ];
        let mut iter = ArangeEntryIter {
            input: EndianSlice::new(&buf, LittleEndian),
            encoding,
        };
        let entry = iter.next();
        assert_eq!(
            iter.input,
            EndianSlice::new(&buf[buf.len() - 1..], LittleEndian)
        );
        assert_eq!(entry, Err(Error::AddressOverflow));
    }

    #[test]
    fn test_parse_entry_overflow_64() {
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 2,
            address_size: 8,
        };
        #[rustfmt::skip]
        let buf = [
            // Address.
            0x01, 0x02, 0x03, 0x04, 0x00, 0x00, 0x00, 0x80,
            // Length.
            0x05, 0x06, 0x07, 0x08, 0x00, 0x00, 0x00, 0x80,
            // Next tuple.
            0x09
        ];
        let mut iter = ArangeEntryIter {
            input: EndianSlice::new(&buf, LittleEndian),
            encoding,
        };
        let entry = iter.next();
        assert_eq!(
            iter.input,
            EndianSlice::new(&buf[buf.len() - 1..], LittleEndian)
        );
        assert_eq!(entry, Err(Error::AddressOverflow));
    }

    #[test]
    fn test_parse_entry_tombstone_32() {
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 2,
            address_size: 4,
        };
        #[rustfmt::skip]
        let buf = [
            // Address.
            0xff, 0xff, 0xff, 0xff,
            // Length.
            0x05, 0x06, 0x07, 0x08,
            // Address.
            0x01, 0x02, 0x03, 0x04,
            // Length.
            0x05, 0x06, 0x07, 0x08,
            // Next tuple.
            0x09
        ];

        let mut iter = ArangeEntryIter {
            input: EndianSlice::new(&buf, LittleEndian),
            encoding,
        };
        let entry = iter.next_raw().unwrap();
        assert_eq!(
            iter.input,
            EndianSlice::new(&buf[buf.len() - 9..], LittleEndian)
        );
        assert_eq!(
            entry,
            Some(ArangeEntry {
                range: Range {
                    begin: 0xffff_ffff,
                    end: 0,
                },
                length: 0x0807_0605,
            })
        );

        let mut iter = ArangeEntryIter {
            input: EndianSlice::new(&buf, LittleEndian),
            encoding,
        };
        let entry = iter.next().unwrap();
        assert_eq!(
            iter.input,
            EndianSlice::new(&buf[buf.len() - 1..], LittleEndian)
        );
        assert_eq!(
            entry,
            Some(ArangeEntry {
                range: Range {
                    begin: 0x0403_0201,
                    end: 0x0403_0201 + 0x0807_0605,
                },
                length: 0x0807_0605,
            })
        );
    }

    #[test]
    fn test_parse_entry_tombstone_64() {
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 2,
            address_size: 8,
        };
        #[rustfmt::skip]
        let buf = [
            // Address.
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            // Length.
            0x05, 0x06, 0x07, 0x08, 0x00, 0x00, 0x00, 0x00,
            // Address.
            0x01, 0x02, 0x03, 0x04, 0x00, 0x00, 0x00, 0x00,
            // Length.
            0x05, 0x06, 0x07, 0x08, 0x00, 0x00, 0x00, 0x00,
            // Next tuple.
            0x09
        ];

        let mut iter = ArangeEntryIter {
            input: EndianSlice::new(&buf, LittleEndian),
            encoding,
        };
        let entry = iter.next_raw().unwrap();
        assert_eq!(
            iter.input,
            EndianSlice::new(&buf[buf.len() - 17..], LittleEndian)
        );
        assert_eq!(
            entry,
            Some(ArangeEntry {
                range: Range {
                    begin: 0xffff_ffff_ffff_ffff,
                    end: 0,
                },
                length: 0x0807_0605,
            })
        );

        let mut iter = ArangeEntryIter {
            input: EndianSlice::new(&buf, LittleEndian),
            encoding,
        };
        let entry = iter.next().unwrap();
        assert_eq!(
            iter.input,
            EndianSlice::new(&buf[buf.len() - 1..], LittleEndian)
        );
        assert_eq!(
            entry,
            Some(ArangeEntry {
                range: Range {
                    begin: 0x0403_0201,
                    end: 0x0403_0201 + 0x0807_0605,
                },
                length: 0x0807_0605,
            })
        );
    }
}
