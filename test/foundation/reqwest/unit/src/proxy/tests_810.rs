#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> http::Uri {
        s.parse().unwrap()
    }

    fn intercepted_uri(p: &Matcher, s: &str) -> Uri {
        p.intercept(&s.parse().unwrap()).unwrap().uri().clone()
    }

    #[test]
    fn test_http() {
        let target = "http://example.domain/";
        let p = Proxy::http(target).unwrap().into_matcher();

        let http = "http://hyper.rs";
        let other = "https://hyper.rs";

        assert_eq!(intercepted_uri(&p, http), target);
        assert!(p.intercept(&url(other)).is_none());
    }

    #[test]
    fn test_https() {
        let target = "http://example.domain/";
        let p = Proxy::https(target).unwrap().into_matcher();

        let http = "http://hyper.rs";
        let other = "https://hyper.rs";

        assert!(p.intercept(&url(http)).is_none());
        assert_eq!(intercepted_uri(&p, other), target);
    }

    #[test]
    fn test_all() {
        let target = "http://example.domain/";
        let p = Proxy::all(target).unwrap().into_matcher();

        let http = "http://hyper.rs";
        let https = "https://hyper.rs";
        // no longer supported
        //let other = "x-youve-never-heard-of-me-mr-proxy://hyper.rs";

        assert_eq!(intercepted_uri(&p, http), target);
        assert_eq!(intercepted_uri(&p, https), target);
        //assert_eq!(intercepted_uri(&p, other), target);
    }

    #[test]
    fn test_custom() {
        let target1 = "http://example.domain/";
        let target2 = "https://example.domain/";
        let p = Proxy::custom(move |url| {
            if url.host_str() == Some("hyper.rs") {
                target1.parse().ok()
            } else if url.scheme() == "http" {
                target2.parse().ok()
            } else {
                None::<Url>
            }
        })
        .into_matcher();

        let http = "http://seanmonstar.com";
        let https = "https://hyper.rs";
        let other = "x-youve-never-heard-of-me-mr-proxy://seanmonstar.com";

        assert_eq!(intercepted_uri(&p, http), target2);
        assert_eq!(intercepted_uri(&p, https), target1);
        assert!(p.intercept(&url(other)).is_none());
    }

    #[test]
    fn test_standard_with_custom_auth_header() {
        let target = "http://example.domain/";
        let p = Proxy::all(target)
            .unwrap()
            .custom_http_auth(http::HeaderValue::from_static("testme"))
            .into_matcher();

        let got = p.intercept(&url("http://anywhere.local")).unwrap();
        let auth = got.basic_auth().unwrap();
        assert_eq!(auth, "testme");
    }

    #[test]
    fn test_custom_with_custom_auth_header() {
        let target = "http://example.domain/";
        let p = Proxy::custom(move |_| target.parse::<Url>().ok())
            .custom_http_auth(http::HeaderValue::from_static("testme"))
            .into_matcher();

        let got = p.intercept(&url("http://anywhere.local")).unwrap();
        let auth = got.basic_auth().unwrap();
        assert_eq!(auth, "testme");
    }

    #[test]
    fn test_maybe_has_http_auth() {
        let m = Proxy::all("https://letme:in@yo.local")
            .unwrap()
            .into_matcher();
        assert!(m.maybe_has_http_auth(), "https forwards");

        let m = Proxy::all("http://letme:in@yo.local")
            .unwrap()
            .into_matcher();
        assert!(m.maybe_has_http_auth(), "http forwards");

        let m = Proxy::all("http://:in@yo.local").unwrap().into_matcher();
        assert!(m.maybe_has_http_auth(), "http forwards with empty username");

        let m = Proxy::all("http://letme:@yo.local").unwrap().into_matcher();
        assert!(m.maybe_has_http_auth(), "http forwards with empty password");
    }

    #[test]
    fn test_socks_proxy_default_port() {
        {
            let m = Proxy::all("socks5://example.com").unwrap().into_matcher();

            let http = "http://hyper.rs";
            let https = "https://hyper.rs";

            assert_eq!(intercepted_uri(&m, http).port_u16(), Some(1080));
            assert_eq!(intercepted_uri(&m, https).port_u16(), Some(1080));

            // custom port
            let m = Proxy::all("socks5://example.com:1234")
                .unwrap()
                .into_matcher();

            assert_eq!(intercepted_uri(&m, http).port_u16(), Some(1234));
            assert_eq!(intercepted_uri(&m, https).port_u16(), Some(1234));
        }
    }
}
