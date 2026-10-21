#[test]
fn json() {
    use crate::subtags::{Language, Region, Script};
    use crate::{langid, locale};

    assert_eq!(
        serde_json::to_string(&langid!("en-US")).unwrap(),
        r#""en-US""#
    );
    assert_eq!(
        serde_json::from_str::<LanguageIdentifier>(r#""en-US""#).unwrap(),
        langid!("en-US")
    );
    assert_eq!(
        serde_json::from_reader::<_, LanguageIdentifier>(&br#""en-US""#[..]).unwrap(),
        langid!("en-US")
    );
    assert!(serde_json::from_str::<LanguageIdentifier>(r#""2Xs""#).is_err());

    assert_eq!(
        serde_json::to_string(&locale!("en-US-u-hc-h12")).unwrap(),
        r#""en-US-u-hc-h12""#
    );
    assert_eq!(
        serde_json::from_str::<Locale>(r#""en-US-u-hc-h12""#).unwrap(),
        locale!("en-US-u-hc-h12")
    );
    assert_eq!(
        serde_json::from_reader::<_, Locale>(&br#""en-US-u-hc-h12""#[..]).unwrap(),
        locale!("en-US-u-hc-h12")
    );
    assert!(serde_json::from_str::<Locale>(r#""2Xs""#).is_err());

    assert_eq!(
        serde_json::to_string(&"fr".parse::<Language>().unwrap()).unwrap(),
        r#""fr""#
    );
    assert_eq!(
        serde_json::from_str::<Language>(r#""fr""#).unwrap(),
        "fr".parse::<Language>().unwrap()
    );
    assert_eq!(
        serde_json::from_reader::<_, Language>(&br#""fr""#[..]).unwrap(),
        "fr".parse::<Language>().unwrap()
    );
    assert!(serde_json::from_str::<Language>(r#""2Xs""#).is_err());

    assert_eq!(
        serde_json::to_string(&"Latn".parse::<Script>().unwrap()).unwrap(),
        r#""Latn""#
    );
    assert_eq!(
        serde_json::from_str::<Script>(r#""Latn""#).unwrap(),
        "Latn".parse::<Script>().unwrap()
    );
    assert_eq!(
        serde_json::from_reader::<_, Script>(&br#""Latn""#[..]).unwrap(),
        "Latn".parse::<Script>().unwrap()
    );
    assert!(serde_json::from_str::<Script>(r#""2Xs""#).is_err());

    assert_eq!(
        serde_json::to_string(&"US".parse::<Region>().unwrap()).unwrap(),
        r#""US""#
    );
    assert_eq!(
        serde_json::from_str::<Region>(r#""US""#).unwrap(),
        "US".parse::<Region>().unwrap()
    );
    assert_eq!(
        serde_json::from_reader::<_, Region>(&br#""US""#[..]).unwrap(),
        "US".parse::<Region>().unwrap()
    );
    assert!(serde_json::from_str::<Region>(r#""2Xs""#).is_err());
}
