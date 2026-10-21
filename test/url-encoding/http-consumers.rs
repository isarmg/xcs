use std::collections::HashMap;
use http_routing::{extract::Query, http::Uri};
use http_transport::adapters::client::proxy::matcher::Matcher;

#[test]
fn routing_query_keeps_form_encoding_semantics() {
    let uri = Uri::from_static("/search?name=%E4%B8%AD%E6%96%87&space=a+b&empty=&duplicate=first&duplicate=last");
    let Query(values) = Query::<HashMap<String, String>>::try_from_uri(&uri).unwrap();
    assert_eq!(values["name"], "中文");
    assert_eq!(values["space"], "a b");
    assert_eq!(values["empty"], "");
    assert_eq!(values["duplicate"], "last");
}

#[test]
fn proxy_credentials_decode_percent_escapes_without_form_plus_rules() {
    let matcher = Matcher::builder()
        .all("socks5h://a%40b:p%3Aq+plus@localhost:1080")
        .build();
    let intercept = matcher.intercept(&Uri::from_static("https://example.com")).unwrap();
    assert_eq!(intercept.raw_auth(), Some(("a@b", "p:q+plus")));
}
