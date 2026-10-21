use unicode_properties::{CodePointMapData, CodePointSetData};
use unicode_properties::props::{Alphabetic, BidiClass, GeneralCategory, Script, WhiteSpace};

#[test]
fn property_vectors_cover_scripts_categories_and_boundaries() {
    let categories = CodePointMapData::<GeneralCategory>::new();
    for (c, expected) in [
        ('A', GeneralCategory::UppercaseLetter),
        ('\u{0301}', GeneralCategory::NonspacingMark),
        ('\u{0665}', GeneralCategory::DecimalNumber),
        ('\u{1f600}', GeneralCategory::OtherSymbol),
        ('\u{10ffff}', GeneralCategory::Unassigned),
    ] { assert_eq!(categories.get(c), expected); }
    let scripts = CodePointMapData::<Script>::new();
    assert_eq!(scripts.get('Ω'), Script::Greek);
    assert_eq!(scripts.get('中'), Script::Han);
    assert_eq!(scripts.get('\u{10400}'), Script::Deseret);
    let bidi = CodePointMapData::<BidiClass>::new();
    assert_eq!(bidi.get('א'), BidiClass::RightToLeft);
    assert_eq!(bidi.get('ش'), BidiClass::ArabicLetter);
    assert!(CodePointSetData::new::<Alphabetic>().contains('Ω'));
    assert!(!CodePointSetData::new::<Alphabetic>().contains('\u{1f600}'));
    assert!(CodePointSetData::new::<WhiteSpace>().contains('\u{2003}'));
    assert!(!CodePointSetData::new::<WhiteSpace>().contains('\u{200b}'));
}

#[test]
fn baked_provider_matches_const_constructors() {
    let supplied = CodePointMapData::<GeneralCategory>::try_new_unstable(
        &unicode_properties::provider::Baked,
    ).unwrap();
    let baked = CodePointMapData::<GeneralCategory>::new();
    let mut checked = 0;
    for scalar in 0..=0x10ffff {
        if let Some(c) = char::from_u32(scalar) {
            assert_eq!(supplied.as_borrowed().get(c), baked.get(c), "U+{scalar:04X}");
            checked += 1;
        }
    }
    assert_eq!(checked, 1_112_064);
}
