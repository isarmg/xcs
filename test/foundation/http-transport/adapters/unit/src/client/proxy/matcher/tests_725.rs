#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_matcher() {
        let domains = vec![".foo.bar".into(), "bar.foo".into()];
        let matcher = DomainMatcher(domains);

        // domains match with leading `.`
        assert!(matcher.contains("foo.bar"));
        assert!(matcher.contains("FOO.BAR"));

        // subdomains match with leading `.`
        assert!(matcher.contains("www.foo.bar"));
        assert!(matcher.contains("WWW.FOO.BAR"));

        // domains match with no leading `.`
        assert!(matcher.contains("bar.foo"));
        assert!(matcher.contains("Bar.foo"));

        // subdomains match with no leading `.`
        assert!(matcher.contains("www.bar.foo"));
        assert!(matcher.contains("WWW.BAR.FOO"));

        // non-subdomain string prefixes don't match
        assert!(!matcher.contains("notfoo.bar"));
        assert!(!matcher.contains("notbar.foo"));
    }

    #[test]
    fn test_no_proxy_wildcard() {
        let no_proxy = NoProxy::from_string("*");
        assert!(no_proxy.contains("any.where"));
    }

    #[test]
    fn test_no_proxy_ip_ranges() {
        let no_proxy =
            NoProxy::from_string(".foo.bar, bar.baz,10.42.1.1/24,::1,10.124.7.8,2001::/17");

        let should_not_match = [
            // random url, not in no_proxy
            "hyper.rs",
            // make sure that random non-subdomain string prefixes don't match
            "notfoo.bar",
            // make sure that random non-subdomain string prefixes don't match
            "notbar.baz",
            // ipv4 address out of range
            "10.43.1.1",
            // ipv4 address out of range
            "10.124.7.7",
            // ipv6 address out of range
            "[ffff:db8:a0b:12f0::1]",
            // ipv6 address out of range
            "[2005:db8:a0b:12f0::1]",
        ];

        for host in &should_not_match {
            assert!(!no_proxy.contains(host), "should not contain {host:?}");
        }

        let should_match = [
            // make sure subdomains (with leading .) match
            "hello.foo.bar",
            // make sure exact matches (without leading .) match (also makes sure spaces between entries work)
            "bar.baz",
            // make sure subdomains (without leading . in no_proxy) match
            "foo.bar.baz",
            // make sure subdomains (without leading . in no_proxy) match - this differs from cURL
            "foo.bar",
            // ipv4 address match within range
            "10.42.1.100",
            // ipv6 address exact match
            "[::1]",
            // ipv6 address match within range
            "[2001:db8:a0b:12f0::1]",
            // ipv4 address exact match
            "10.124.7.8",
        ];

        for host in &should_match {
            assert!(no_proxy.contains(host), "should contain {host:?}");
        }
    }

    #[cfg(all(feature = "client-proxy-system", windows))]
    #[test]
    fn test_windows_proxy_override_ip_wildcard() {
        let normalized =
            win::normalize_proxy_override("127.*; 10.*.*.*; 192.168.*; 192.168.1.*; *.example.com");
        let no_proxy = NoProxy::from_string(&normalized);

        assert!(no_proxy.contains("127.0.0.1"));
        assert!(no_proxy.contains("10.12.34.56"));
        assert!(no_proxy.contains("192.168.42.1"));
        assert!(no_proxy.contains("192.168.1.42"));
        assert!(no_proxy.contains("www.example.com"));
        assert!(!no_proxy.contains("128.0.0.1"));
        assert!(!no_proxy.contains("192.169.42.1"));

        let subnet = NoProxy::from_string(&win::normalize_proxy_override("192.168.1.*"));
        assert!(!subnet.contains("192.168.2.42"));
    }

    macro_rules! p {
        ($($n:ident = $v:expr,)*) => ({Builder {
            $($n: $v.into(),)*
            ..Builder::default()
        }.build()});
    }

    fn intercept(p: &Matcher, u: &str) -> Intercept {
        p.intercept(&u.parse().unwrap()).unwrap()
    }

    #[test]
    fn test_all_proxy() {
        let p = p! {
            all = "http://om.nom",
        };

        assert_eq!("http://om.nom", intercept(&p, "http://example.com").uri());

        assert_eq!("http://om.nom", intercept(&p, "https://example.com").uri());
    }

    #[test]
    fn test_specific_overrides_all() {
        let p = p! {
            all = "http://no.pe",
            http = "http://y.ep",
        };

        assert_eq!("http://no.pe", intercept(&p, "https://example.com").uri());

        // the http rule is "more specific" than the all rule
        assert_eq!("http://y.ep", intercept(&p, "http://example.com").uri());
    }

    #[test]
    fn test_parse_no_scheme_defaults_to_http() {
        let p = p! {
            https = "y.ep",
            http = "127.0.0.1:8887",
        };

        assert_eq!(intercept(&p, "https://example.local").uri(), "http://y.ep");
        assert_eq!(
            intercept(&p, "http://example.local").uri(),
            "http://127.0.0.1:8887"
        );
    }

    #[test]
    fn test_parse_http_auth() {
        let p = p! {
            all = "http://Aladdin:opensesame@y.ep",
        };

        let proxy = intercept(&p, "https://example.local");
        assert_eq!(proxy.uri(), "http://y.ep");
        assert_eq!(
            proxy.basic_auth().expect("basic_auth"),
            "Basic QWxhZGRpbjpvcGVuc2VzYW1l"
        );
    }

    #[test]
    fn test_parse_http_auth_without_password() {
        let p = p! {
            all = "http://Aladdin@y.ep",
        };
        let proxy = intercept(&p, "https://example.local");
        assert_eq!(proxy.uri(), "http://y.ep");
        assert_eq!(
            proxy.basic_auth().expect("basic_auth"),
            "Basic QWxhZGRpbjo="
        );
    }

    #[test]
    fn test_parse_http_auth_without_scheme() {
        let p = p! {
            all = "Aladdin:opensesame@y.ep",
        };

        let proxy = intercept(&p, "https://example.local");
        assert_eq!(proxy.uri(), "http://y.ep");
        assert_eq!(
            proxy.basic_auth().expect("basic_auth"),
            "Basic QWxhZGRpbjpvcGVuc2VzYW1l"
        );
    }

    #[test]
    fn test_dont_parse_http_when_is_cgi() {
        let mut builder = Matcher::builder();
        builder.is_cgi = true;
        builder.http = "http://never.gonna.let.you.go".into();
        let m = builder.build();

        assert!(m.intercept(&"http://rick.roll".parse().unwrap()).is_none());
    }

    #[test]
    fn test_domain_matcher_case_insensitive() {
        let domains = vec![".foo.bar".into()];
        let matcher = DomainMatcher(domains);

        assert!(matcher.contains("foo.bar"));
        assert!(matcher.contains("FOO.BAR"));
        assert!(matcher.contains("Foo.Bar"));

        assert!(matcher.contains("www.foo.bar"));
        assert!(matcher.contains("WWW.FOO.BAR"));
        assert!(matcher.contains("Www.Foo.Bar"));
    }

    #[test]
    fn test_no_proxy_case_insensitive() {
        let p = p! {
            all = "http://proxy.local",
            no = ".example.com",
        };

        // should bypass proxy (case insensitive match)
        assert!(
            p.intercept(&"http://example.com".parse().unwrap())
                .is_none()
        );
        assert!(
            p.intercept(&"http://EXAMPLE.COM".parse().unwrap())
                .is_none()
        );
        assert!(
            p.intercept(&"http://Example.com".parse().unwrap())
                .is_none()
        );

        // subdomain should bypass proxy (case insensitive match)
        assert!(
            p.intercept(&"http://www.example.com".parse().unwrap())
                .is_none()
        );
        assert!(
            p.intercept(&"http://WWW.EXAMPLE.COM".parse().unwrap())
                .is_none()
        );
        assert!(
            p.intercept(&"http://Www.Example.Com".parse().unwrap())
                .is_none()
        );
    }
}
