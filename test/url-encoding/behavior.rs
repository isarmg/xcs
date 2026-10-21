use url_encoding::{address::Url, domain, form, percent};
#[test]
fn byte_roundtrips_and_invalid_utf8() {
    for byte in 0..=255 {
        let raw = [byte];
        let encoded = percent::percent_encode(&raw, percent::NON_ALPHANUMERIC).to_string();
        assert_eq!(percent::percent_decode_str(&encoded).collect::<Vec<_>>(), raw);
    }
    assert!(percent::percent_decode_str("%FF").decode_utf8().is_err());
    assert_eq!(percent::percent_decode_str("%FF").decode_utf8_lossy(), "\u{fffd}");
    assert_eq!(percent::percent_decode_str("%Q1%").collect::<Vec<_>>(), b"%Q1%");
    assert_eq!(form::parse(b"a+b=c%2Bd&empty=&bad=%FF").collect::<Vec<_>>(), vec![("a b".into(), "c+d".into()), ("empty".into(), "".into()), ("bad".into(), "\u{fffd}".into())]);
    let encoded = form::Serializer::new(String::new()).append_pair("空 格", "x+y&z").finish();
    assert_eq!(form::parse(encoded.as_bytes()).next().unwrap(), ("空 格".into(), "x+y&z".into()));
}
#[test]
fn canonical_urls_and_security_boundaries() {
    let u=Url::parse("HTTPS://BÜCHER.example:443/a/../b?q=空 格#x").unwrap();
    assert_eq!(u.as_str(), "https://xn--bcher-kva.example/b?q=%E7%A9%BA%20%E6%A0%BC#x");
    assert_eq!(Url::parse("http://0x7f.1/").unwrap().host_str(), Some("127.0.0.1"));
    assert_eq!(Url::parse("https://[2001:0db8::1]:443/").unwrap().as_str(), "https://[2001:db8::1]/");
    assert!(Url::parse("https://a%00b/").is_err());
    assert!(Url::parse("https://\u{200d}.example/").is_err());
    assert_eq!(domain::domain_to_ascii("faß.de").unwrap(), "xn--fa-hia.de");
    assert_eq!(domain::domain_to_ascii("日本語。ＪＰ").unwrap(), "xn--wgv71a119e.jp");
    assert!(domain::domain_to_ascii("a\u{05d0}.example").is_err());
    assert_eq!(u.join("../next").unwrap().path(), "/next");
}
#[test]
fn retained_punycode_vectors() {
    let rows: Vec<json_codec::Value> = json_codec::from_str(include_str!("../foundation/url-encoding/domain/tests/punycode_tests.json")).unwrap();
    assert!(rows.len()>20);
    for row in rows {
        let decoded=row["decoded"].as_str().unwrap();
        let encoded=row["encoded"].as_str().unwrap();
        assert_eq!(domain::punycode::encode_str(decoded).unwrap(), encoded);
        assert_eq!(domain::punycode::decode_to_string(encoded).unwrap(), decoded);
    }
}
