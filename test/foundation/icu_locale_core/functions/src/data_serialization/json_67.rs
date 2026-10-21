#[test]
fn json() {
    use crate::subtags::{Language, Region, Script};
    use crate::{langid, locale};

    assert_eq!(
        json_codec::to_string(&langid!("en-US")).unwrap(),
        r#""en-US""#
    );
    assert_eq!(
        json_codec::from_str::<LanguageIdentifier>(r#""en-US""#).unwrap(),
        langid!("en-US")
    );
    assert_eq!(
        json_codec::from_reader::<_, LanguageIdentifier>(&br#""en-US""#[..]).unwrap(),
        langid!("en-US")
    );
    assert!(json_codec::from_str::<LanguageIdentifier>(r#""2Xs""#).is_err());

    assert_eq!(
        json_codec::to_string(&locale!("en-US-u-hc-h12")).unwrap(),
        r#""en-US-u-hc-h12""#
    );
    assert_eq!(
        json_codec::from_str::<Locale>(r#""en-US-u-hc-h12""#).unwrap(),
        locale!("en-US-u-hc-h12")
    );
    assert_eq!(
        json_codec::from_reader::<_, Locale>(&br#""en-US-u-hc-h12""#[..]).unwrap(),
        locale!("en-US-u-hc-h12")
    );
    assert!(json_codec::from_str::<Locale>(r#""2Xs""#).is_err());

    assert_eq!(
        json_codec::to_string(&"fr".parse::<Language>().unwrap()).unwrap(),
        r#""fr""#
    );
    assert_eq!(
        json_codec::from_str::<Language>(r#""fr""#).unwrap(),
        "fr".parse::<Language>().unwrap()
    );
    assert_eq!(
        json_codec::from_reader::<_, Language>(&br#""fr""#[..]).unwrap(),
        "fr".parse::<Language>().unwrap()
    );
    assert!(json_codec::from_str::<Language>(r#""2Xs""#).is_err());

    assert_eq!(
        json_codec::to_string(&"Latn".parse::<Script>().unwrap()).unwrap(),
        r#""Latn""#
    );
    assert_eq!(
        json_codec::from_str::<Script>(r#""Latn""#).unwrap(),
        "Latn".parse::<Script>().unwrap()
    );
    assert_eq!(
        json_codec::from_reader::<_, Script>(&br#""Latn""#[..]).unwrap(),
        "Latn".parse::<Script>().unwrap()
    );
    assert!(json_codec::from_str::<Script>(r#""2Xs""#).is_err());

    assert_eq!(
        json_codec::to_string(&"US".parse::<Region>().unwrap()).unwrap(),
        r#""US""#
    );
    assert_eq!(
        json_codec::from_str::<Region>(r#""US""#).unwrap(),
        "US".parse::<Region>().unwrap()
    );
    assert_eq!(
        json_codec::from_reader::<_, Region>(&br#""US""#[..]).unwrap(),
        "US".parse::<Region>().unwrap()
    );
    assert!(json_codec::from_str::<Region>(r#""2Xs""#).is_err());
}
