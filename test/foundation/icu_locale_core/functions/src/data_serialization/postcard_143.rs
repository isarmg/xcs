#[test]
fn postcard() {
    use crate::subtags::{Language, Region, Script};
    use crate::{langid, locale};

    assert_eq!(
        postcard::to_stdvec(&langid!("en-US")).unwrap(),
        b"\x05en-US"
    );
    assert_eq!(
        postcard::from_bytes::<LanguageIdentifier>(b"\x05en-US").unwrap(),
        langid!("en-US")
    );
    assert!(postcard::from_bytes::<LanguageIdentifier>(b"\x032Xs").is_err());

    assert_eq!(
        postcard::to_stdvec(&locale!("en-US-u-hc-h12")).unwrap(),
        b"\x0Een-US-u-hc-h12"
    );
    assert_eq!(
        postcard::from_bytes::<Locale>(b"\x0Een-US-u-hc-h12").unwrap(),
        locale!("en-US-u-hc-h12")
    );
    assert!(postcard::from_bytes::<Locale>(b"\x032Xs").is_err());

    assert_eq!(
        postcard::to_stdvec(&"fr".parse::<Language>().unwrap()).unwrap(),
        b"fr\0"
    );
    assert_eq!(
        postcard::from_bytes::<Language>(b"fr\0").unwrap(),
        "fr".parse::<Language>().unwrap()
    );
    assert!(postcard::from_bytes::<Language>(b"2Xs").is_err());

    assert_eq!(
        postcard::to_stdvec(&"Latn".parse::<Script>().unwrap()).unwrap(),
        b"Latn"
    );
    assert_eq!(
        postcard::from_bytes::<Script>(b"Latn").unwrap(),
        "Latn".parse::<Script>().unwrap()
    );
    assert!(postcard::from_bytes::<Script>(b"2Xss").is_err());

    assert_eq!(
        postcard::to_stdvec(&"US".parse::<Region>().unwrap()).unwrap(),
        b"US\0"
    );
    assert_eq!(
        postcard::from_bytes::<Region>(b"US\0").unwrap(),
        "US".parse::<Region>().unwrap()
    );
    assert!(postcard::from_bytes::<Region>(b"2Xs").is_err());
}
