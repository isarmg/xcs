#[cfg(all(test, feature = "compiled_data"))]
mod tests {
    use super::*;

    #[test]
    #[expect(deprecated, reason = "compare every Unicode scalar against the original ICU4C bit representation")]
    fn property_masks_preserve_all_unicode_scalars() {
        let bidi = icu_properties::CodePointMapData::<icu_properties::props::BidiClass>::new();
        let joining = icu_properties::CodePointMapData::<icu_properties::props::JoiningType>::new();
        let mut checked = 0;
        for scalar in 0..=0x10ffff {
            let Some(c) = char::from_u32(scalar) else { continue };
            let bc = bidi.get(c);
            let jt = joining.get(c);
            assert_eq!(bidi_class_to_mask(bc), 1u32 << bc.to_icu4c_value(), "U+{scalar:04X}");
            assert_eq!(joining_type_to_mask(jt), 1u32 << jt.to_icu4c_value(), "U+{scalar:04X}");
            checked += 1;
        }
        assert_eq!(checked, 1_112_064);
    }
}
