#[test]
// Expected sizes are based on a 64-bit architecture
#[cfg(target_pointer_width = "64")]
fn test_sizes() {
    assert_eq!(size_of::<subtags::Language>(), 3);
    assert_eq!(size_of::<subtags::Script>(), 4);
    assert_eq!(size_of::<subtags::Region>(), 3);
    assert_eq!(size_of::<subtags::Variant>(), 8);
    assert_eq!(size_of::<subtags::Variants>(), 16);
    assert_eq!(size_of::<LanguageIdentifier>(), 32);

    assert_eq!(size_of::<extensions::transform::Transform>(), 56);
    assert_eq!(size_of::<Option<LanguageIdentifier>>(), 32);
    assert_eq!(size_of::<extensions::transform::Fields>(), 24);

    assert_eq!(size_of::<extensions::unicode::Attributes>(), 16);
    assert_eq!(size_of::<extensions::unicode::Keywords>(), 24);
    assert_eq!(size_of::<Vec<extensions::other::Other>>(), 24);
    assert_eq!(size_of::<extensions::private::Private>(), 16);
    assert_eq!(size_of::<extensions::Extensions>(), 136);

    assert_eq!(size_of::<Locale>(), 168);
}
