#[cfg(test)]
mod tests {
    use super::*;
    use crate::write;
    use crate::{BigEndian, LittleEndian};

    #[test]
    fn test_writer() {
        let mut w = write::EndianVec::new(LittleEndian);
        w.write_address(Address::Constant(0x1122_3344), 4).unwrap();
        assert_eq!(w.slice(), &[0x44, 0x33, 0x22, 0x11]);
        assert_eq!(
            w.write_address(
                Address::Symbol {
                    symbol: 0,
                    addend: 0
                },
                4
            ),
            Err(Error::InvalidAddress)
        );

        let mut w = write::EndianVec::new(LittleEndian);
        w.write_offset(0x1122_3344, SectionId::DebugInfo, 4)
            .unwrap();
        assert_eq!(w.slice(), &[0x44, 0x33, 0x22, 0x11]);
        w.write_offset_at(1, 0x5566, SectionId::DebugInfo, 2)
            .unwrap();
        assert_eq!(w.slice(), &[0x44, 0x66, 0x55, 0x11]);

        let mut w = write::EndianVec::new(LittleEndian);
        w.write_u8(0x11).unwrap();
        w.write_u16(0x2233).unwrap();
        w.write_u32(0x4455_6677).unwrap();
        w.write_u64(0x8081_8283_8485_8687).unwrap();
        #[rustfmt::skip]
        assert_eq!(w.slice(), &[
            0x11,
            0x33, 0x22,
            0x77, 0x66, 0x55, 0x44,
            0x87, 0x86, 0x85, 0x84, 0x83, 0x82, 0x81, 0x80,
        ]);
        w.write_u8_at(14, 0x11).unwrap();
        w.write_u16_at(12, 0x2233).unwrap();
        w.write_u32_at(8, 0x4455_6677).unwrap();
        w.write_u64_at(0, 0x8081_8283_8485_8687).unwrap();
        #[rustfmt::skip]
        assert_eq!(w.slice(), &[
            0x87, 0x86, 0x85, 0x84, 0x83, 0x82, 0x81, 0x80,
            0x77, 0x66, 0x55, 0x44,
            0x33, 0x22,
            0x11,
        ]);

        let mut w = write::EndianVec::new(BigEndian);
        w.write_u8(0x11).unwrap();
        w.write_u16(0x2233).unwrap();
        w.write_u32(0x4455_6677).unwrap();
        w.write_u64(0x8081_8283_8485_8687).unwrap();
        #[rustfmt::skip]
        assert_eq!(w.slice(), &[
            0x11,
            0x22, 0x33,
            0x44, 0x55, 0x66, 0x77,
            0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87,
        ]);
        w.write_u8_at(14, 0x11).unwrap();
        w.write_u16_at(12, 0x2233).unwrap();
        w.write_u32_at(8, 0x4455_6677).unwrap();
        w.write_u64_at(0, 0x8081_8283_8485_8687).unwrap();
        #[rustfmt::skip]
        assert_eq!(w.slice(), &[
            0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87,
            0x44, 0x55, 0x66, 0x77,
            0x22, 0x33,
            0x11,
        ]);

        let mut w = write::EndianVec::new(LittleEndian);
        w.write_udata(0x11, 1).unwrap();
        w.write_udata(0x2233, 2).unwrap();
        w.write_udata(0x4455_6677, 4).unwrap();
        w.write_udata(0x8081_8283_8485_8687, 8).unwrap();
        #[rustfmt::skip]
        assert_eq!(w.slice(), &[
            0x11,
            0x33, 0x22,
            0x77, 0x66, 0x55, 0x44,
            0x87, 0x86, 0x85, 0x84, 0x83, 0x82, 0x81, 0x80,
        ]);
        assert_eq!(w.write_udata(0x100, 1), Err(Error::ValueTooLarge));
        assert_eq!(w.write_udata(0x1_0000, 2), Err(Error::ValueTooLarge));
        assert_eq!(w.write_udata(0x1_0000_0000, 4), Err(Error::ValueTooLarge));
        assert_eq!(w.write_udata(0x00, 3), Err(Error::UnsupportedWordSize(3)));
        w.write_udata_at(14, 0x11, 1).unwrap();
        w.write_udata_at(12, 0x2233, 2).unwrap();
        w.write_udata_at(8, 0x4455_6677, 4).unwrap();
        w.write_udata_at(0, 0x8081_8283_8485_8687, 8).unwrap();
        #[rustfmt::skip]
        assert_eq!(w.slice(), &[
            0x87, 0x86, 0x85, 0x84, 0x83, 0x82, 0x81, 0x80,
            0x77, 0x66, 0x55, 0x44,
            0x33, 0x22,
            0x11,
        ]);
        assert_eq!(w.write_udata_at(0, 0x100, 1), Err(Error::ValueTooLarge));
        assert_eq!(w.write_udata_at(0, 0x1_0000, 2), Err(Error::ValueTooLarge));
        assert_eq!(
            w.write_udata_at(0, 0x1_0000_0000, 4),
            Err(Error::ValueTooLarge)
        );
        assert_eq!(
            w.write_udata_at(0, 0x00, 3),
            Err(Error::UnsupportedWordSize(3))
        );

        let mut w = write::EndianVec::new(LittleEndian);
        w.write_uleb128(0).unwrap();
        assert_eq!(w.slice(), &[0]);

        let mut w = write::EndianVec::new(LittleEndian);
        w.write_uleb128(u64::MAX).unwrap();
        assert_eq!(
            w.slice(),
            &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 1]
        );

        let mut w = write::EndianVec::new(LittleEndian);
        w.write_sleb128(0).unwrap();
        assert_eq!(w.slice(), &[0]);

        let mut w = write::EndianVec::new(LittleEndian);
        w.write_sleb128(i64::MAX).unwrap();
        assert_eq!(
            w.slice(),
            &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0]
        );

        let mut w = write::EndianVec::new(LittleEndian);
        w.write_sleb128(i64::MIN).unwrap();
        assert_eq!(
            w.slice(),
            &[0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x7f]
        );

        let mut w = write::EndianVec::new(LittleEndian);
        let offset = w.write_initial_length(Format::Dwarf32).unwrap();
        assert_eq!(w.slice(), &[0, 0, 0, 0]);
        w.write_initial_length_at(offset, 0x1122_3344, Format::Dwarf32)
            .unwrap();
        assert_eq!(w.slice(), &[0x44, 0x33, 0x22, 0x11]);
        assert_eq!(
            w.write_initial_length_at(offset, 0x1_0000_0000, Format::Dwarf32),
            Err(Error::ValueTooLarge)
        );

        let mut w = write::EndianVec::new(LittleEndian);
        let offset = w.write_initial_length(Format::Dwarf64).unwrap();
        assert_eq!(w.slice(), &[0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 0]);
        w.write_initial_length_at(offset, 0x1122_3344_5566_7788, Format::Dwarf64)
            .unwrap();
        assert_eq!(
            w.slice(),
            &[0xff, 0xff, 0xff, 0xff, 0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11]
        );
    }
}
