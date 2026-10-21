#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{DebugAbbrevOffset, DebugInfoOffset, Encoding, Format};
    use crate::endianity::LittleEndian;
    use crate::read::{
        Abbreviation, AttributeSpecification, DebuggingInformationEntry, EndianSlice, UnitHeader,
        UnitOffset, UnitType,
    };

    #[test]
    #[rustfmt::skip]
    fn valuetype_from_encoding() {
        let encoding = Encoding {
            format: Format::Dwarf32,
            version: 4,
            address_size: 4,
        };
        let unit = UnitHeader::new(
            encoding,
            7,
            UnitType::Compilation,
            DebugAbbrevOffset(0),
            DebugInfoOffset(0).into(),
            EndianSlice::new(&[], LittleEndian),
        );

        let abbrev = Abbreviation::new(
            42,
            constants::DW_TAG_base_type,
            constants::DW_CHILDREN_no,
            vec![
                AttributeSpecification::new(
                    constants::DW_AT_byte_size,
                    constants::DW_FORM_udata,
                    None,
                ),
                AttributeSpecification::new(
                    constants::DW_AT_encoding,
                    constants::DW_FORM_udata,
                    None,
                ),
                AttributeSpecification::new(
                    constants::DW_AT_endianity,
                    constants::DW_FORM_udata,
                    None,
                ),
            ].into(),
        );

        for &(attrs, result) in &[
            ([0x01, constants::DW_ATE_signed.0, constants::DW_END_default.0], ValueType::I8),
            ([0x02, constants::DW_ATE_signed.0, constants::DW_END_default.0], ValueType::I16),
            ([0x04, constants::DW_ATE_signed.0, constants::DW_END_default.0], ValueType::I32),
            ([0x08, constants::DW_ATE_signed.0, constants::DW_END_default.0], ValueType::I64),
            ([0x01, constants::DW_ATE_unsigned.0, constants::DW_END_default.0], ValueType::U8),
            ([0x02, constants::DW_ATE_unsigned.0, constants::DW_END_default.0], ValueType::U16),
            ([0x04, constants::DW_ATE_unsigned.0, constants::DW_END_default.0], ValueType::U32),
            ([0x08, constants::DW_ATE_unsigned.0, constants::DW_END_default.0], ValueType::U64),
            ([0x04, constants::DW_ATE_float.0, constants::DW_END_default.0], ValueType::F32),
            ([0x08, constants::DW_ATE_float.0, constants::DW_END_default.0], ValueType::F64),
        ] {
            let entry = DebuggingInformationEntry::new(
                UnitOffset(0),
                EndianSlice::new(&attrs, LittleEndian),
                &abbrev,
                &unit,
            );
            assert_eq!(ValueType::from_entry(&entry), Ok(Some(result)));
        }

        for attrs in &[
            [0x03, constants::DW_ATE_signed.0, constants::DW_END_default.0],
            [0x02, constants::DW_ATE_signed.0, constants::DW_END_big.0],
        ] {
            let entry = DebuggingInformationEntry::new(
                UnitOffset(0),
                EndianSlice::new(attrs, LittleEndian),
                &abbrev,
                &unit,
            );
            assert_eq!(ValueType::from_entry(&entry), Ok(None));
        }
    }

    #[test]
    fn value_convert() {
        let addr_mask = !0 >> 32;
        for &(v, t, result) in &[
            (Value::Generic(1), ValueType::I8, Ok(Value::I8(1))),
            (Value::I8(1), ValueType::U8, Ok(Value::U8(1))),
            (Value::U8(1), ValueType::I16, Ok(Value::I16(1))),
            (Value::I16(1), ValueType::U16, Ok(Value::U16(1))),
            (Value::U16(1), ValueType::I32, Ok(Value::I32(1))),
            (Value::I32(1), ValueType::U32, Ok(Value::U32(1))),
            (Value::U32(1), ValueType::F32, Ok(Value::F32(1.))),
            (Value::F32(1.), ValueType::I64, Ok(Value::I64(1))),
            (Value::I64(1), ValueType::U64, Ok(Value::U64(1))),
            (Value::U64(1), ValueType::F64, Ok(Value::F64(1.))),
            (Value::F64(1.), ValueType::Generic, Ok(Value::Generic(1))),
        ] {
            assert_eq!(v.convert(t, addr_mask), result);
        }
    }

    #[test]
    #[rustfmt::skip]
    fn value_reinterpret() {
        let addr_mask = !0 >> 32;
        for &(v, t, result) in &[
            // 8-bit
            (Value::I8(-1), ValueType::U8, Ok(Value::U8(0xff))),
            (Value::U8(0xff), ValueType::I8, Ok(Value::I8(-1))),
            // 16-bit
            (Value::I16(1), ValueType::U16, Ok(Value::U16(1))),
            (Value::U16(1), ValueType::I16, Ok(Value::I16(1))),
            // 32-bit
            (Value::Generic(1), ValueType::I32, Ok(Value::I32(1))),
            (Value::I32(1), ValueType::U32, Ok(Value::U32(1))),
            (Value::U32(0x3f80_0000), ValueType::F32, Ok(Value::F32(1.0))),
            (Value::F32(1.0), ValueType::Generic, Ok(Value::Generic(0x3f80_0000))),
            // Type mismatches
            (Value::Generic(1), ValueType::U8, Err(Error::TypeMismatch)),
            (Value::U8(1), ValueType::U16, Err(Error::TypeMismatch)),
            (Value::U16(1), ValueType::U32, Err(Error::TypeMismatch)),
            (Value::U32(1), ValueType::U64, Err(Error::TypeMismatch)),
            (Value::U64(1), ValueType::Generic, Err(Error::TypeMismatch)),
        ] {
            assert_eq!(v.reinterpret(t, addr_mask), result);
        }

        let addr_mask = !0;
        for &(v, t, result) in &[
            // 64-bit
            (Value::Generic(1), ValueType::I64, Ok(Value::I64(1))),
            (Value::I64(1), ValueType::U64, Ok(Value::U64(1))),
            (Value::U64(0x3ff0_0000_0000_0000), ValueType::F64, Ok(Value::F64(1.0))),
            (Value::F64(1.0), ValueType::Generic, Ok(Value::Generic(0x3ff0_0000_0000_0000))),
        ] {
            assert_eq!(v.reinterpret(t, addr_mask), result);
        }
    }

    #[test]
    #[rustfmt::skip]
    fn value_abs() {
        let addr_mask = 0xffff_ffff;
        for &(v, result) in &[
            (Value::Generic(0xffff_ffff), Ok(Value::Generic(1))),
            (Value::I8(-1), Ok(Value::I8(1))),
            (Value::U8(1), Ok(Value::U8(1))),
            (Value::I16(-1), Ok(Value::I16(1))),
            (Value::U16(1), Ok(Value::U16(1))),
            (Value::I32(-1), Ok(Value::I32(1))),
            (Value::U32(1), Ok(Value::U32(1))),
            (Value::I64(-1), Ok(Value::I64(1))),
            (Value::U64(1), Ok(Value::U64(1))),
            (Value::F32(-1.), Ok(Value::F32(1.))),
            (Value::F64(-1.), Ok(Value::F64(1.))),
        ] {
            assert_eq!(v.abs(addr_mask), result);
        }
    }

    #[test]
    #[rustfmt::skip]
    fn value_neg() {
        let addr_mask = 0xffff_ffff;
        for &(v, result) in &[
            (Value::Generic(0xffff_ffff), Ok(Value::Generic(1))),
            (Value::I8(1), Ok(Value::I8(-1))),
            (Value::U8(1), Err(Error::UnsupportedTypeOperation)),
            (Value::I16(1), Ok(Value::I16(-1))),
            (Value::U16(1), Err(Error::UnsupportedTypeOperation)),
            (Value::I32(1), Ok(Value::I32(-1))),
            (Value::U32(1), Err(Error::UnsupportedTypeOperation)),
            (Value::I64(1), Ok(Value::I64(-1))),
            (Value::U64(1), Err(Error::UnsupportedTypeOperation)),
            (Value::F32(1.), Ok(Value::F32(-1.))),
            (Value::F64(1.), Ok(Value::F64(-1.))),
        ] {
            assert_eq!(v.neg(addr_mask), result);
        }
    }

    #[test]
    #[rustfmt::skip]
    fn value_add() {
        let addr_mask = 0xffff_ffff;
        for &(v1, v2, result) in &[
            (Value::Generic(1), Value::Generic(2), Ok(Value::Generic(3))),
            (Value::I8(-1), Value::I8(2), Ok(Value::I8(1))),
            (Value::U8(1), Value::U8(2), Ok(Value::U8(3))),
            (Value::I16(-1), Value::I16(2), Ok(Value::I16(1))),
            (Value::U16(1), Value::U16(2), Ok(Value::U16(3))),
            (Value::I32(-1), Value::I32(2), Ok(Value::I32(1))),
            (Value::U32(1), Value::U32(2), Ok(Value::U32(3))),
            (Value::I64(-1), Value::I64(2), Ok(Value::I64(1))),
            (Value::U64(1), Value::U64(2), Ok(Value::U64(3))),
            (Value::F32(-1.), Value::F32(2.), Ok(Value::F32(1.))),
            (Value::F64(-1.), Value::F64(2.), Ok(Value::F64(1.))),
            (Value::Generic(1), Value::U32(2), Err(Error::TypeMismatch)),
        ] {
            assert_eq!(v1.add(v2, addr_mask), result);
        }
    }

    #[test]
    #[rustfmt::skip]
    fn value_sub() {
        let addr_mask = 0xffff_ffff;
        for &(v1, v2, result) in &[
            (Value::Generic(3), Value::Generic(2), Ok(Value::Generic(1))),
            (Value::I8(-1), Value::I8(2), Ok(Value::I8(-3))),
            (Value::U8(3), Value::U8(2), Ok(Value::U8(1))),
            (Value::I16(-1), Value::I16(2), Ok(Value::I16(-3))),
            (Value::U16(3), Value::U16(2), Ok(Value::U16(1))),
            (Value::I32(-1), Value::I32(2), Ok(Value::I32(-3))),
            (Value::U32(3), Value::U32(2), Ok(Value::U32(1))),
            (Value::I64(-1), Value::I64(2), Ok(Value::I64(-3))),
            (Value::U64(3), Value::U64(2), Ok(Value::U64(1))),
            (Value::F32(-1.), Value::F32(2.), Ok(Value::F32(-3.))),
            (Value::F64(-1.), Value::F64(2.), Ok(Value::F64(-3.))),
            (Value::Generic(3), Value::U32(2), Err(Error::TypeMismatch)),
        ] {
            assert_eq!(v1.sub(v2, addr_mask), result);
        }
    }

    #[test]
    #[rustfmt::skip]
    fn value_mul() {
        let addr_mask = 0xffff_ffff;
        for &(v1, v2, result) in &[
            (Value::Generic(2), Value::Generic(3), Ok(Value::Generic(6))),
            (Value::I8(-2), Value::I8(3), Ok(Value::I8(-6))),
            (Value::U8(2), Value::U8(3), Ok(Value::U8(6))),
            (Value::I16(-2), Value::I16(3), Ok(Value::I16(-6))),
            (Value::U16(2), Value::U16(3), Ok(Value::U16(6))),
            (Value::I32(-2), Value::I32(3), Ok(Value::I32(-6))),
            (Value::U32(2), Value::U32(3), Ok(Value::U32(6))),
            (Value::I64(-2), Value::I64(3), Ok(Value::I64(-6))),
            (Value::U64(2), Value::U64(3), Ok(Value::U64(6))),
            (Value::F32(-2.), Value::F32(3.), Ok(Value::F32(-6.))),
            (Value::F64(-2.), Value::F64(3.), Ok(Value::F64(-6.))),
            (Value::Generic(2), Value::U32(3), Err(Error::TypeMismatch)),
        ] {
            assert_eq!(v1.mul(v2, addr_mask), result);
        }
    }

    #[test]
    #[rustfmt::skip]
    fn value_div() {
        let addr_mask = 0xffff_ffff;
        for &(v1, v2, result) in &[
            (Value::Generic(6), Value::Generic(3), Ok(Value::Generic(2))),
            (Value::I8(-6), Value::I8(3), Ok(Value::I8(-2))),
            (Value::U8(6), Value::U8(3), Ok(Value::U8(2))),
            (Value::I16(-6), Value::I16(3), Ok(Value::I16(-2))),
            (Value::U16(6), Value::U16(3), Ok(Value::U16(2))),
            (Value::I32(-6), Value::I32(3), Ok(Value::I32(-2))),
            (Value::U32(6), Value::U32(3), Ok(Value::U32(2))),
            (Value::I64(-6), Value::I64(3), Ok(Value::I64(-2))),
            (Value::U64(6), Value::U64(3), Ok(Value::U64(2))),
            (Value::F32(-6.), Value::F32(3.), Ok(Value::F32(-2.))),
            (Value::F64(-6.), Value::F64(3.), Ok(Value::F64(-2.))),
            (Value::Generic(6), Value::U32(3), Err(Error::TypeMismatch)),
        ] {
            assert_eq!(v1.div(v2, addr_mask), result);
        }
        for &(v1, v2, result) in &[
            (Value::Generic(6), Value::Generic(0), Err(Error::DivisionByZero)),
            (Value::I8(-6), Value::I8(0), Err(Error::DivisionByZero)),
            (Value::U8(6), Value::U8(0), Err(Error::DivisionByZero)),
            (Value::I16(-6), Value::I16(0), Err(Error::DivisionByZero)),
            (Value::U16(6), Value::U16(0), Err(Error::DivisionByZero)),
            (Value::I32(-6), Value::I32(0), Err(Error::DivisionByZero)),
            (Value::U32(6), Value::U32(0), Err(Error::DivisionByZero)),
            (Value::I64(-6), Value::I64(0), Err(Error::DivisionByZero)),
            (Value::U64(6), Value::U64(0), Err(Error::DivisionByZero)),
            (Value::F32(-6.), Value::F32(0.), Ok(Value::F32(-6. / 0.))),
            (Value::F64(-6.), Value::F64(0.), Ok(Value::F64(-6. / 0.))),
        ] {
            assert_eq!(v1.div(v2, addr_mask), result);
        }
    }

    #[test]
    #[rustfmt::skip]
    fn value_rem() {
        let addr_mask = 0xffff_ffff;
        for &(v1, v2, result) in &[
            (Value::Generic(3), Value::Generic(2), Ok(Value::Generic(1))),
            (Value::I8(-3), Value::I8(2), Ok(Value::I8(-1))),
            (Value::U8(3), Value::U8(2), Ok(Value::U8(1))),
            (Value::I16(-3), Value::I16(2), Ok(Value::I16(-1))),
            (Value::U16(3), Value::U16(2), Ok(Value::U16(1))),
            (Value::I32(-3), Value::I32(2), Ok(Value::I32(-1))),
            (Value::U32(3), Value::U32(2), Ok(Value::U32(1))),
            (Value::I64(-3), Value::I64(2), Ok(Value::I64(-1))),
            (Value::U64(3), Value::U64(2), Ok(Value::U64(1))),
            (Value::F32(-3.), Value::F32(2.), Err(Error::IntegralTypeRequired)),
            (Value::F64(-3.), Value::F64(2.), Err(Error::IntegralTypeRequired)),
            (Value::Generic(3), Value::U32(2), Err(Error::TypeMismatch)),
        ] {
            assert_eq!(v1.rem(v2, addr_mask), result);
        }
        for &(v1, v2, result) in &[
            (Value::Generic(3), Value::Generic(0), Err(Error::DivisionByZero)),
            (Value::I8(-3), Value::I8(0), Err(Error::DivisionByZero)),
            (Value::U8(3), Value::U8(0), Err(Error::DivisionByZero)),
            (Value::I16(-3), Value::I16(0), Err(Error::DivisionByZero)),
            (Value::U16(3), Value::U16(0), Err(Error::DivisionByZero)),
            (Value::I32(-3), Value::I32(0), Err(Error::DivisionByZero)),
            (Value::U32(3), Value::U32(0), Err(Error::DivisionByZero)),
            (Value::I64(-3), Value::I64(0), Err(Error::DivisionByZero)),
            (Value::U64(3), Value::U64(0), Err(Error::DivisionByZero)),
        ] {
            assert_eq!(v1.rem(v2, addr_mask), result);
        }
    }

    #[test]
    #[rustfmt::skip]
    fn value_not() {
        let addr_mask = 0xffff_ffff;
        for &(v, result) in &[
            (Value::Generic(1), Ok(Value::Generic(!1))),
            (Value::I8(1), Ok(Value::I8(!1))),
            (Value::U8(1), Ok(Value::U8(!1))),
            (Value::I16(1), Ok(Value::I16(!1))),
            (Value::U16(1), Ok(Value::U16(!1))),
            (Value::I32(1), Ok(Value::I32(!1))),
            (Value::U32(1), Ok(Value::U32(!1))),
            (Value::I64(1), Ok(Value::I64(!1))),
            (Value::U64(1), Ok(Value::U64(!1))),
            (Value::F32(1.), Err(Error::IntegralTypeRequired)),
            (Value::F64(1.), Err(Error::IntegralTypeRequired)),
        ] {
            assert_eq!(v.not(addr_mask), result);
        }
    }

    #[test]
    #[rustfmt::skip]
    fn value_and() {
        let addr_mask = 0xffff_ffff;
        for &(v1, v2, result) in &[
            (Value::Generic(3), Value::Generic(5), Ok(Value::Generic(1))),
            (Value::I8(3), Value::I8(5), Ok(Value::I8(1))),
            (Value::U8(3), Value::U8(5), Ok(Value::U8(1))),
            (Value::I16(3), Value::I16(5), Ok(Value::I16(1))),
            (Value::U16(3), Value::U16(5), Ok(Value::U16(1))),
            (Value::I32(3), Value::I32(5), Ok(Value::I32(1))),
            (Value::U32(3), Value::U32(5), Ok(Value::U32(1))),
            (Value::I64(3), Value::I64(5), Ok(Value::I64(1))),
            (Value::U64(3), Value::U64(5), Ok(Value::U64(1))),
            (Value::F32(3.), Value::F32(5.), Err(Error::IntegralTypeRequired)),
            (Value::F64(3.), Value::F64(5.), Err(Error::IntegralTypeRequired)),
            (Value::Generic(3), Value::U32(5), Err(Error::TypeMismatch)),
        ] {
            assert_eq!(v1.and(v2, addr_mask), result);
        }
    }

    #[test]
    #[rustfmt::skip]
    fn value_or() {
        let addr_mask = 0xffff_ffff;
        for &(v1, v2, result) in &[
            (Value::Generic(3), Value::Generic(5), Ok(Value::Generic(7))),
            (Value::I8(3), Value::I8(5), Ok(Value::I8(7))),
            (Value::U8(3), Value::U8(5), Ok(Value::U8(7))),
            (Value::I16(3), Value::I16(5), Ok(Value::I16(7))),
            (Value::U16(3), Value::U16(5), Ok(Value::U16(7))),
            (Value::I32(3), Value::I32(5), Ok(Value::I32(7))),
            (Value::U32(3), Value::U32(5), Ok(Value::U32(7))),
            (Value::I64(3), Value::I64(5), Ok(Value::I64(7))),
            (Value::U64(3), Value::U64(5), Ok(Value::U64(7))),
            (Value::F32(3.), Value::F32(5.), Err(Error::IntegralTypeRequired)),
            (Value::F64(3.), Value::F64(5.), Err(Error::IntegralTypeRequired)),
            (Value::Generic(3), Value::U32(5), Err(Error::TypeMismatch)),
        ] {
            assert_eq!(v1.or(v2, addr_mask), result);
        }
    }

    #[test]
    #[rustfmt::skip]
    fn value_xor() {
        let addr_mask = 0xffff_ffff;
        for &(v1, v2, result) in &[
            (Value::Generic(3), Value::Generic(5), Ok(Value::Generic(6))),
            (Value::I8(3), Value::I8(5), Ok(Value::I8(6))),
            (Value::U8(3), Value::U8(5), Ok(Value::U8(6))),
            (Value::I16(3), Value::I16(5), Ok(Value::I16(6))),
            (Value::U16(3), Value::U16(5), Ok(Value::U16(6))),
            (Value::I32(3), Value::I32(5), Ok(Value::I32(6))),
            (Value::U32(3), Value::U32(5), Ok(Value::U32(6))),
            (Value::I64(3), Value::I64(5), Ok(Value::I64(6))),
            (Value::U64(3), Value::U64(5), Ok(Value::U64(6))),
            (Value::F32(3.), Value::F32(5.), Err(Error::IntegralTypeRequired)),
            (Value::F64(3.), Value::F64(5.), Err(Error::IntegralTypeRequired)),
            (Value::Generic(3), Value::U32(5), Err(Error::TypeMismatch)),
        ] {
            assert_eq!(v1.xor(v2, addr_mask), result);
        }
    }

    #[test]
    #[rustfmt::skip]
    fn value_shl() {
        let addr_mask = 0xffff_ffff;
        for &(v1, v2, result) in &[
            // One of each type
            (Value::Generic(3), Value::Generic(5), Ok(Value::Generic(96))),
            (Value::I8(3), Value::U8(5), Ok(Value::I8(96))),
            (Value::U8(3), Value::I8(5), Ok(Value::U8(96))),
            (Value::I16(3), Value::U16(5), Ok(Value::I16(96))),
            (Value::U16(3), Value::I16(5), Ok(Value::U16(96))),
            (Value::I32(3), Value::U32(5), Ok(Value::I32(96))),
            (Value::U32(3), Value::I32(5), Ok(Value::U32(96))),
            (Value::I64(3), Value::U64(5), Ok(Value::I64(96))),
            (Value::U64(3), Value::I64(5), Ok(Value::U64(96))),
            (Value::F32(3.), Value::U8(5), Err(Error::IntegralTypeRequired)),
            (Value::F64(3.), Value::U8(5), Err(Error::IntegralTypeRequired)),
            // Invalid shifts
            (Value::U8(3), Value::I8(-5), Err(Error::InvalidShiftExpression)),
            (Value::U8(3), Value::I16(-5), Err(Error::InvalidShiftExpression)),
            (Value::U8(3), Value::I32(-5), Err(Error::InvalidShiftExpression)),
            (Value::U8(3), Value::I64(-5), Err(Error::InvalidShiftExpression)),
            (Value::U8(3), Value::F32(5.), Err(Error::InvalidShiftExpression)),
            (Value::U8(3), Value::F64(5.), Err(Error::InvalidShiftExpression)),
            // Large shifts
            (Value::Generic(3), Value::Generic(32), Ok(Value::Generic(0))),
            (Value::I8(3), Value::U8(8), Ok(Value::I8(0))),
            (Value::U8(3), Value::I8(9), Ok(Value::U8(0))),
            (Value::I16(3), Value::U16(17), Ok(Value::I16(0))),
            (Value::U16(3), Value::I16(16), Ok(Value::U16(0))),
            (Value::I32(3), Value::U32(32), Ok(Value::I32(0))),
            (Value::U32(3), Value::I32(33), Ok(Value::U32(0))),
            (Value::I64(3), Value::U64(65), Ok(Value::I64(0))),
            (Value::U64(3), Value::I64(64), Ok(Value::U64(0))),
        ] {
            assert_eq!(v1.shl(v2, addr_mask), result);
        }
    }

    #[test]
    #[rustfmt::skip]
    fn value_shr() {
        let addr_mask = 0xffff_ffff;
        for &(v1, v2, result) in &[
            // One of each type
            (Value::Generic(96), Value::Generic(5), Ok(Value::Generic(3))),
            (Value::I8(96), Value::U8(5), Err(Error::UnsupportedTypeOperation)),
            (Value::U8(96), Value::I8(5), Ok(Value::U8(3))),
            (Value::I16(96), Value::U16(5), Err(Error::UnsupportedTypeOperation)),
            (Value::U16(96), Value::I16(5), Ok(Value::U16(3))),
            (Value::I32(96), Value::U32(5), Err(Error::UnsupportedTypeOperation)),
            (Value::U32(96), Value::I32(5), Ok(Value::U32(3))),
            (Value::I64(96), Value::U64(5), Err(Error::UnsupportedTypeOperation)),
            (Value::U64(96), Value::I64(5), Ok(Value::U64(3))),
            (Value::F32(96.), Value::U8(5), Err(Error::IntegralTypeRequired)),
            (Value::F64(96.), Value::U8(5), Err(Error::IntegralTypeRequired)),
            // Invalid shifts
            (Value::U8(96), Value::I8(-5), Err(Error::InvalidShiftExpression)),
            (Value::U8(96), Value::I16(-5), Err(Error::InvalidShiftExpression)),
            (Value::U8(96), Value::I32(-5), Err(Error::InvalidShiftExpression)),
            (Value::U8(96), Value::I64(-5), Err(Error::InvalidShiftExpression)),
            (Value::U8(96), Value::F32(5.), Err(Error::InvalidShiftExpression)),
            (Value::U8(96), Value::F64(5.), Err(Error::InvalidShiftExpression)),
            // Large shifts
            (Value::Generic(96), Value::Generic(32), Ok(Value::Generic(0))),
            (Value::U8(96), Value::I8(9), Ok(Value::U8(0))),
            (Value::U16(96), Value::I16(16), Ok(Value::U16(0))),
            (Value::U32(96), Value::I32(33), Ok(Value::U32(0))),
            (Value::U64(96), Value::I64(64), Ok(Value::U64(0))),
        ] {
            assert_eq!(v1.shr(v2, addr_mask), result);
        }
    }

    #[test]
    #[rustfmt::skip]
    fn value_shra() {
        let addr_mask = 0xffff_ffff;
        for &(v1, v2, result) in &[
            // One of each type
            (Value::Generic(u64::from(-96i32 as u32)), Value::Generic(5), Ok(Value::Generic(-3i64 as u64))),
            (Value::I8(-96), Value::U8(5), Ok(Value::I8(-3))),
            (Value::U8(96), Value::I8(5), Err(Error::UnsupportedTypeOperation)),
            (Value::I16(-96), Value::U16(5), Ok(Value::I16(-3))),
            (Value::U16(96), Value::I16(5), Err(Error::UnsupportedTypeOperation)),
            (Value::I32(-96), Value::U32(5), Ok(Value::I32(-3))),
            (Value::U32(96), Value::I32(5), Err(Error::UnsupportedTypeOperation)),
            (Value::I64(-96), Value::U64(5), Ok(Value::I64(-3))),
            (Value::U64(96), Value::I64(5), Err(Error::UnsupportedTypeOperation)),
            (Value::F32(96.), Value::U8(5), Err(Error::IntegralTypeRequired)),
            (Value::F64(96.), Value::U8(5), Err(Error::IntegralTypeRequired)),
            // Invalid shifts
            (Value::U8(96), Value::I8(-5), Err(Error::InvalidShiftExpression)),
            (Value::U8(96), Value::I16(-5), Err(Error::InvalidShiftExpression)),
            (Value::U8(96), Value::I32(-5), Err(Error::InvalidShiftExpression)),
            (Value::U8(96), Value::I64(-5), Err(Error::InvalidShiftExpression)),
            (Value::U8(96), Value::F32(5.), Err(Error::InvalidShiftExpression)),
            (Value::U8(96), Value::F64(5.), Err(Error::InvalidShiftExpression)),
            // Large shifts
            (Value::Generic(96), Value::Generic(32), Ok(Value::Generic(0))),
            (Value::I8(96), Value::U8(8), Ok(Value::I8(0))),
            (Value::I8(-96), Value::U8(8), Ok(Value::I8(-1))),
            (Value::I16(96), Value::U16(17), Ok(Value::I16(0))),
            (Value::I16(-96), Value::U16(17), Ok(Value::I16(-1))),
            (Value::I32(96), Value::U32(32), Ok(Value::I32(0))),
            (Value::I32(-96), Value::U32(32), Ok(Value::I32(-1))),
            (Value::I64(96), Value::U64(65), Ok(Value::I64(0))),
            (Value::I64(-96), Value::U64(65), Ok(Value::I64(-1))),
        ] {
            assert_eq!(v1.shra(v2, addr_mask), result);
        }
    }

    #[test]
    fn value_eq() {
        let addr_mask = 0xffff_ffff;
        for &(v1, v2, result) in &[
            (Value::Generic(3), Value::Generic(3), Ok(Value::Generic(1))),
            (Value::Generic(!3), Value::Generic(3), Ok(Value::Generic(0))),
            (Value::I8(3), Value::I8(3), Ok(Value::Generic(1))),
            (Value::I8(!3), Value::I8(3), Ok(Value::Generic(0))),
            (Value::U8(3), Value::U8(3), Ok(Value::Generic(1))),
            (Value::U8(!3), Value::U8(3), Ok(Value::Generic(0))),
            (Value::I16(3), Value::I16(3), Ok(Value::Generic(1))),
            (Value::I16(!3), Value::I16(3), Ok(Value::Generic(0))),
            (Value::U16(3), Value::U16(3), Ok(Value::Generic(1))),
            (Value::U16(!3), Value::U16(3), Ok(Value::Generic(0))),
            (Value::I32(3), Value::I32(3), Ok(Value::Generic(1))),
            (Value::I32(!3), Value::I32(3), Ok(Value::Generic(0))),
            (Value::U32(3), Value::U32(3), Ok(Value::Generic(1))),
            (Value::U32(!3), Value::U32(3), Ok(Value::Generic(0))),
            (Value::I64(3), Value::I64(3), Ok(Value::Generic(1))),
            (Value::I64(!3), Value::I64(3), Ok(Value::Generic(0))),
            (Value::U64(3), Value::U64(3), Ok(Value::Generic(1))),
            (Value::U64(!3), Value::U64(3), Ok(Value::Generic(0))),
            (Value::F32(3.), Value::F32(3.), Ok(Value::Generic(1))),
            (Value::F32(-3.), Value::F32(3.), Ok(Value::Generic(0))),
            (Value::F64(3.), Value::F64(3.), Ok(Value::Generic(1))),
            (Value::F64(-3.), Value::F64(3.), Ok(Value::Generic(0))),
            (Value::Generic(3), Value::U32(3), Err(Error::TypeMismatch)),
        ] {
            assert_eq!(v1.eq(v2, addr_mask), result);
        }
    }

    #[test]
    fn value_ne() {
        let addr_mask = 0xffff_ffff;
        for &(v1, v2, result) in &[
            (Value::Generic(3), Value::Generic(3), Ok(Value::Generic(0))),
            (Value::Generic(!3), Value::Generic(3), Ok(Value::Generic(1))),
            (Value::I8(3), Value::I8(3), Ok(Value::Generic(0))),
            (Value::I8(!3), Value::I8(3), Ok(Value::Generic(1))),
            (Value::U8(3), Value::U8(3), Ok(Value::Generic(0))),
            (Value::U8(!3), Value::U8(3), Ok(Value::Generic(1))),
            (Value::I16(3), Value::I16(3), Ok(Value::Generic(0))),
            (Value::I16(!3), Value::I16(3), Ok(Value::Generic(1))),
            (Value::U16(3), Value::U16(3), Ok(Value::Generic(0))),
            (Value::U16(!3), Value::U16(3), Ok(Value::Generic(1))),
            (Value::I32(3), Value::I32(3), Ok(Value::Generic(0))),
            (Value::I32(!3), Value::I32(3), Ok(Value::Generic(1))),
            (Value::U32(3), Value::U32(3), Ok(Value::Generic(0))),
            (Value::U32(!3), Value::U32(3), Ok(Value::Generic(1))),
            (Value::I64(3), Value::I64(3), Ok(Value::Generic(0))),
            (Value::I64(!3), Value::I64(3), Ok(Value::Generic(1))),
            (Value::U64(3), Value::U64(3), Ok(Value::Generic(0))),
            (Value::U64(!3), Value::U64(3), Ok(Value::Generic(1))),
            (Value::F32(3.), Value::F32(3.), Ok(Value::Generic(0))),
            (Value::F32(-3.), Value::F32(3.), Ok(Value::Generic(1))),
            (Value::F64(3.), Value::F64(3.), Ok(Value::Generic(0))),
            (Value::F64(-3.), Value::F64(3.), Ok(Value::Generic(1))),
            (Value::Generic(3), Value::U32(3), Err(Error::TypeMismatch)),
        ] {
            assert_eq!(v1.ne(v2, addr_mask), result);
        }
    }

    #[test]
    fn value_ge() {
        let addr_mask = 0xffff_ffff;
        for &(v1, v2, result) in &[
            (Value::Generic(3), Value::Generic(!3), Ok(Value::Generic(1))),
            (Value::Generic(!3), Value::Generic(3), Ok(Value::Generic(0))),
            (Value::I8(3), Value::I8(!3), Ok(Value::Generic(1))),
            (Value::I8(!3), Value::I8(3), Ok(Value::Generic(0))),
            (Value::U8(3), Value::U8(!3), Ok(Value::Generic(0))),
            (Value::U8(!3), Value::U8(3), Ok(Value::Generic(1))),
            (Value::I16(3), Value::I16(!3), Ok(Value::Generic(1))),
            (Value::I16(!3), Value::I16(3), Ok(Value::Generic(0))),
            (Value::U16(3), Value::U16(!3), Ok(Value::Generic(0))),
            (Value::U16(!3), Value::U16(3), Ok(Value::Generic(1))),
            (Value::I32(3), Value::I32(!3), Ok(Value::Generic(1))),
            (Value::I32(!3), Value::I32(3), Ok(Value::Generic(0))),
            (Value::U32(3), Value::U32(!3), Ok(Value::Generic(0))),
            (Value::U32(!3), Value::U32(3), Ok(Value::Generic(1))),
            (Value::I64(3), Value::I64(!3), Ok(Value::Generic(1))),
            (Value::I64(!3), Value::I64(3), Ok(Value::Generic(0))),
            (Value::U64(3), Value::U64(!3), Ok(Value::Generic(0))),
            (Value::U64(!3), Value::U64(3), Ok(Value::Generic(1))),
            (Value::F32(3.), Value::F32(-3.), Ok(Value::Generic(1))),
            (Value::F32(-3.), Value::F32(3.), Ok(Value::Generic(0))),
            (Value::F64(3.), Value::F64(-3.), Ok(Value::Generic(1))),
            (Value::F64(-3.), Value::F64(3.), Ok(Value::Generic(0))),
            (Value::Generic(3), Value::U32(3), Err(Error::TypeMismatch)),
        ] {
            assert_eq!(v1.ge(v2, addr_mask), result);
        }
    }

    #[test]
    fn value_gt() {
        let addr_mask = 0xffff_ffff;
        for &(v1, v2, result) in &[
            (Value::Generic(3), Value::Generic(!3), Ok(Value::Generic(1))),
            (Value::Generic(!3), Value::Generic(3), Ok(Value::Generic(0))),
            (Value::I8(3), Value::I8(!3), Ok(Value::Generic(1))),
            (Value::I8(!3), Value::I8(3), Ok(Value::Generic(0))),
            (Value::U8(3), Value::U8(!3), Ok(Value::Generic(0))),
            (Value::U8(!3), Value::U8(3), Ok(Value::Generic(1))),
            (Value::I16(3), Value::I16(!3), Ok(Value::Generic(1))),
            (Value::I16(!3), Value::I16(3), Ok(Value::Generic(0))),
            (Value::U16(3), Value::U16(!3), Ok(Value::Generic(0))),
            (Value::U16(!3), Value::U16(3), Ok(Value::Generic(1))),
            (Value::I32(3), Value::I32(!3), Ok(Value::Generic(1))),
            (Value::I32(!3), Value::I32(3), Ok(Value::Generic(0))),
            (Value::U32(3), Value::U32(!3), Ok(Value::Generic(0))),
            (Value::U32(!3), Value::U32(3), Ok(Value::Generic(1))),
            (Value::I64(3), Value::I64(!3), Ok(Value::Generic(1))),
            (Value::I64(!3), Value::I64(3), Ok(Value::Generic(0))),
            (Value::U64(3), Value::U64(!3), Ok(Value::Generic(0))),
            (Value::U64(!3), Value::U64(3), Ok(Value::Generic(1))),
            (Value::F32(3.), Value::F32(-3.), Ok(Value::Generic(1))),
            (Value::F32(-3.), Value::F32(3.), Ok(Value::Generic(0))),
            (Value::F64(3.), Value::F64(-3.), Ok(Value::Generic(1))),
            (Value::F64(-3.), Value::F64(3.), Ok(Value::Generic(0))),
            (Value::Generic(3), Value::U32(3), Err(Error::TypeMismatch)),
        ] {
            assert_eq!(v1.gt(v2, addr_mask), result);
        }
    }

    #[test]
    fn value_le() {
        let addr_mask = 0xffff_ffff;
        for &(v1, v2, result) in &[
            (Value::Generic(3), Value::Generic(!3), Ok(Value::Generic(0))),
            (Value::Generic(!3), Value::Generic(3), Ok(Value::Generic(1))),
            (Value::I8(3), Value::I8(!3), Ok(Value::Generic(0))),
            (Value::I8(!3), Value::I8(3), Ok(Value::Generic(1))),
            (Value::U8(3), Value::U8(!3), Ok(Value::Generic(1))),
            (Value::U8(!3), Value::U8(3), Ok(Value::Generic(0))),
            (Value::I16(3), Value::I16(!3), Ok(Value::Generic(0))),
            (Value::I16(!3), Value::I16(3), Ok(Value::Generic(1))),
            (Value::U16(3), Value::U16(!3), Ok(Value::Generic(1))),
            (Value::U16(!3), Value::U16(3), Ok(Value::Generic(0))),
            (Value::I32(3), Value::I32(!3), Ok(Value::Generic(0))),
            (Value::I32(!3), Value::I32(3), Ok(Value::Generic(1))),
            (Value::U32(3), Value::U32(!3), Ok(Value::Generic(1))),
            (Value::U32(!3), Value::U32(3), Ok(Value::Generic(0))),
            (Value::I64(3), Value::I64(!3), Ok(Value::Generic(0))),
            (Value::I64(!3), Value::I64(3), Ok(Value::Generic(1))),
            (Value::U64(3), Value::U64(!3), Ok(Value::Generic(1))),
            (Value::U64(!3), Value::U64(3), Ok(Value::Generic(0))),
            (Value::F32(3.), Value::F32(-3.), Ok(Value::Generic(0))),
            (Value::F32(-3.), Value::F32(3.), Ok(Value::Generic(1))),
            (Value::F64(3.), Value::F64(-3.), Ok(Value::Generic(0))),
            (Value::F64(-3.), Value::F64(3.), Ok(Value::Generic(1))),
            (Value::Generic(3), Value::U32(3), Err(Error::TypeMismatch)),
        ] {
            assert_eq!(v1.le(v2, addr_mask), result);
        }
    }

    #[test]
    fn value_lt() {
        let addr_mask = 0xffff_ffff;
        for &(v1, v2, result) in &[
            (Value::Generic(3), Value::Generic(!3), Ok(Value::Generic(0))),
            (Value::Generic(!3), Value::Generic(3), Ok(Value::Generic(1))),
            (Value::I8(3), Value::I8(!3), Ok(Value::Generic(0))),
            (Value::I8(!3), Value::I8(3), Ok(Value::Generic(1))),
            (Value::U8(3), Value::U8(!3), Ok(Value::Generic(1))),
            (Value::U8(!3), Value::U8(3), Ok(Value::Generic(0))),
            (Value::I16(3), Value::I16(!3), Ok(Value::Generic(0))),
            (Value::I16(!3), Value::I16(3), Ok(Value::Generic(1))),
            (Value::U16(3), Value::U16(!3), Ok(Value::Generic(1))),
            (Value::U16(!3), Value::U16(3), Ok(Value::Generic(0))),
            (Value::I32(3), Value::I32(!3), Ok(Value::Generic(0))),
            (Value::I32(!3), Value::I32(3), Ok(Value::Generic(1))),
            (Value::U32(3), Value::U32(!3), Ok(Value::Generic(1))),
            (Value::U32(!3), Value::U32(3), Ok(Value::Generic(0))),
            (Value::I64(3), Value::I64(!3), Ok(Value::Generic(0))),
            (Value::I64(!3), Value::I64(3), Ok(Value::Generic(1))),
            (Value::U64(3), Value::U64(!3), Ok(Value::Generic(1))),
            (Value::U64(!3), Value::U64(3), Ok(Value::Generic(0))),
            (Value::F32(3.), Value::F32(-3.), Ok(Value::Generic(0))),
            (Value::F32(-3.), Value::F32(3.), Ok(Value::Generic(1))),
            (Value::F64(3.), Value::F64(-3.), Ok(Value::Generic(0))),
            (Value::F64(-3.), Value::F64(3.), Ok(Value::Generic(1))),
            (Value::Generic(3), Value::U32(3), Err(Error::TypeMismatch)),
        ] {
            assert_eq!(v1.lt(v2, addr_mask), result);
        }
    }
}
