#[cfg(feature = "export")]
#[test]
fn test_iter() {
    use crate::IterableDataProvider;
    use icu_locale_core::data_locale;

    let ids = HelloWorldProvider.iter_ids().unwrap();

    assert_eq!(ids.len(), HelloWorldProvider::DATA.len());

    assert!(ids.contains(&DataIdentifierCow::from_borrowed_and_owned(
        DataMarkerAttributes::from_str_or_panic("reverse"),
        data_locale!("en")
    )));
}
