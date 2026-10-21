#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::type_complexity)]
    const PRESENTED_MATCHES_REFERENCE: &[(&[u8], &[u8], Result<bool, Error>)] = &[
        (b"", b"a", Err(Error::MalformedDnsIdentifier)),
        (b"a", b"a", Ok(true)),
        (b"b", b"a", Ok(false)),
        (b"*.b.a", b"c.b.a", Ok(true)),
        (b"*.b.a", b"b.a", Ok(false)),
        (b"*.b.a", b"b.a.", Ok(false)),
        // Wildcard not in leftmost label
        (b"d.c.b.a", b"d.c.b.a", Ok(true)),
        (b"d.*.b.a", b"d.c.b.a", Err(Error::MalformedDnsIdentifier)),
        (b"d.c*.b.a", b"d.c.b.a", Err(Error::MalformedDnsIdentifier)),
        (b"d.c*.b.a", b"d.cc.b.a", Err(Error::MalformedDnsIdentifier)),
        // case sensitivity
        (
            b"abcdefghijklmnopqrstuvwxyz",
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZ",
            Ok(true),
        ),
        (
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZ",
            b"abcdefghijklmnopqrstuvwxyz",
            Ok(true),
        ),
        (b"aBc", b"Abc", Ok(true)),
        // digits
        (b"a1", b"a1", Ok(true)),
        // A trailing dot indicates an absolute name, and absolute names can match
        // relative names, and vice-versa.
        (b"example", b"example", Ok(true)),
        (b"example.", b"example.", Err(Error::MalformedDnsIdentifier)),
        (b"example", b"example.", Ok(true)),
        (b"example.", b"example", Err(Error::MalformedDnsIdentifier)),
        (b"example.com", b"example.com", Ok(true)),
        (
            b"example.com.",
            b"example.com.",
            Err(Error::MalformedDnsIdentifier),
        ),
        (b"example.com", b"example.com.", Ok(true)),
        (
            b"example.com.",
            b"example.com",
            Err(Error::MalformedDnsIdentifier),
        ),
        (
            b"example.com..",
            b"example.com.",
            Err(Error::MalformedDnsIdentifier),
        ),
        (
            b"example.com..",
            b"example.com",
            Err(Error::MalformedDnsIdentifier),
        ),
        (
            b"example.com...",
            b"example.com.",
            Err(Error::MalformedDnsIdentifier),
        ),
        // xn-- IDN prefix
        (b"x*.b.a", b"xa.b.a", Err(Error::MalformedDnsIdentifier)),
        (b"x*.b.a", b"xna.b.a", Err(Error::MalformedDnsIdentifier)),
        (b"x*.b.a", b"xn-a.b.a", Err(Error::MalformedDnsIdentifier)),
        (b"x*.b.a", b"xn--a.b.a", Err(Error::MalformedDnsIdentifier)),
        (b"xn*.b.a", b"xn--a.b.a", Err(Error::MalformedDnsIdentifier)),
        (
            b"xn-*.b.a",
            b"xn--a.b.a",
            Err(Error::MalformedDnsIdentifier),
        ),
        (
            b"xn--*.b.a",
            b"xn--a.b.a",
            Err(Error::MalformedDnsIdentifier),
        ),
        (b"xn*.b.a", b"xn--a.b.a", Err(Error::MalformedDnsIdentifier)),
        (
            b"xn-*.b.a",
            b"xn--a.b.a",
            Err(Error::MalformedDnsIdentifier),
        ),
        (
            b"xn--*.b.a",
            b"xn--a.b.a",
            Err(Error::MalformedDnsIdentifier),
        ),
        (
            b"xn---*.b.a",
            b"xn--a.b.a",
            Err(Error::MalformedDnsIdentifier),
        ),
        // "*" cannot expand to nothing.
        (b"c*.b.a", b"c.b.a", Err(Error::MalformedDnsIdentifier)),
        // --------------------------------------------------------------------------
        // The rest of these are test cases adapted from Chromium's
        // x509_certificate_unittest.cc. The parameter order is the opposite in
        // Chromium's tests. Also, they Ok tests were modified to fit into this
        // framework or due to intentional differences between mozilla::pkix and
        // Chromium.
        (b"foo.com", b"foo.com", Ok(true)),
        (b"f", b"f", Ok(true)),
        (b"i", b"h", Ok(false)),
        (b"*.foo.com", b"bar.foo.com", Ok(true)),
        (b"*.test.fr", b"www.test.fr", Ok(true)),
        (b"*.test.FR", b"wwW.tESt.fr", Ok(true)),
        (b".uk", b"f.uk", Err(Error::MalformedDnsIdentifier)),
        (
            b"?.bar.foo.com",
            b"w.bar.foo.com",
            Err(Error::MalformedDnsIdentifier),
        ),
        (
            b"(www|ftp).foo.com",
            b"www.foo.com",
            Err(Error::MalformedDnsIdentifier),
        ), // regex!
        (
            b"www.foo.com\0",
            b"www.foo.com",
            Err(Error::MalformedDnsIdentifier),
        ),
        (
            b"www.foo.com\0*.foo.com",
            b"www.foo.com",
            Err(Error::MalformedDnsIdentifier),
        ),
        (b"ww.house.example", b"www.house.example", Ok(false)),
        (b"www.test.org", b"test.org", Ok(false)),
        (b"*.test.org", b"test.org", Ok(false)),
        (b"*.org", b"test.org", Err(Error::MalformedDnsIdentifier)),
        // '*' must be the only character in the wildcard label
        (
            b"w*.bar.foo.com",
            b"w.bar.foo.com",
            Err(Error::MalformedDnsIdentifier),
        ),
        (
            b"ww*ww.bar.foo.com",
            b"www.bar.foo.com",
            Err(Error::MalformedDnsIdentifier),
        ),
        (
            b"ww*ww.bar.foo.com",
            b"wwww.bar.foo.com",
            Err(Error::MalformedDnsIdentifier),
        ),
        (
            b"w*w.bar.foo.com",
            b"wwww.bar.foo.com",
            Err(Error::MalformedDnsIdentifier),
        ),
        (
            b"w*w.bar.foo.c0m",
            b"wwww.bar.foo.com",
            Err(Error::MalformedDnsIdentifier),
        ),
        (
            b"wa*.bar.foo.com",
            b"WALLY.bar.foo.com",
            Err(Error::MalformedDnsIdentifier),
        ),
        (
            b"*Ly.bar.foo.com",
            b"wally.bar.foo.com",
            Err(Error::MalformedDnsIdentifier),
        ),
        // Chromium does URL decoding of the reference ID, but we don't, and we also
        // require that the reference ID is valid, so we can't test these two.
        //     (b"www.foo.com", b"ww%57.foo.com", Ok(true)),
        //     (b"www&.foo.com", b"www%26.foo.com", Ok(true)),
        (b"*.test.de", b"www.test.co.jp", Ok(false)),
        (
            b"*.jp",
            b"www.test.co.jp",
            Err(Error::MalformedDnsIdentifier),
        ),
        (b"www.test.co.uk", b"www.test.co.jp", Ok(false)),
        (
            b"www.*.co.jp",
            b"www.test.co.jp",
            Err(Error::MalformedDnsIdentifier),
        ),
        (b"www.bar.foo.com", b"www.bar.foo.com", Ok(true)),
        (b"*.foo.com", b"www.bar.foo.com", Ok(false)),
        (
            b"*.*.foo.com",
            b"www.bar.foo.com",
            Err(Error::MalformedDnsIdentifier),
        ),
        // Our matcher requires the reference ID to be a valid DNS name, so we cannot
        // test this case.
        //     (b"*.*.bar.foo.com", b"*..bar.foo.com", Ok(false)),
        (b"www.bath.org", b"www.bath.org", Ok(true)),
        // Our matcher requires the reference ID to be a valid DNS name, so we cannot
        // test these cases.
        // DNS_ID_MISMATCH("www.bath.org", ""),
        //     (b"www.bath.org", b"20.30.40.50", Ok(false)),
        //     (b"www.bath.org", b"66.77.88.99", Ok(false)),

        // IDN tests
        (
            b"xn--poema-9qae5a.com.br",
            b"xn--poema-9qae5a.com.br",
            Ok(true),
        ),
        (
            b"*.xn--poema-9qae5a.com.br",
            b"www.xn--poema-9qae5a.com.br",
            Ok(true),
        ),
        (
            b"*.xn--poema-9qae5a.com.br",
            b"xn--poema-9qae5a.com.br",
            Ok(false),
        ),
        (
            b"xn--poema-*.com.br",
            b"xn--poema-9qae5a.com.br",
            Err(Error::MalformedDnsIdentifier),
        ),
        (
            b"xn--*-9qae5a.com.br",
            b"xn--poema-9qae5a.com.br",
            Err(Error::MalformedDnsIdentifier),
        ),
        (
            b"*--poema-9qae5a.com.br",
            b"xn--poema-9qae5a.com.br",
            Err(Error::MalformedDnsIdentifier),
        ),
        // The following are adapted from the examples quoted from
        //   http://tools.ietf.org/html/rfc6125#section-6.4.3
        // (e.g., *.example.com would match foo.example.com but
        // not bar.foo.example.com or example.com).
        (b"*.example.com", b"foo.example.com", Ok(true)),
        (b"*.example.com", b"bar.foo.example.com", Ok(false)),
        (b"*.example.com", b"example.com", Ok(false)),
        (
            b"baz*.example.net",
            b"baz1.example.net",
            Err(Error::MalformedDnsIdentifier),
        ),
        (
            b"*baz.example.net",
            b"foobaz.example.net",
            Err(Error::MalformedDnsIdentifier),
        ),
        (
            b"b*z.example.net",
            b"buzz.example.net",
            Err(Error::MalformedDnsIdentifier),
        ),
        // Wildcards should not be valid for public registry controlled domains,
        // and unknown/unrecognized domains, at least three domain components must
        // be present. For mozilla::pkix and NSS, there must always be at least two
        // labels after the wildcard label.
        (b"*.test.example", b"www.test.example", Ok(true)),
        (b"*.example.co.uk", b"test.example.co.uk", Ok(true)),
        (
            b"*.example",
            b"test.example",
            Err(Error::MalformedDnsIdentifier),
        ),
        // The result is different than Chromium, because Chromium takes into account
        // the additional knowledge it has that "co.uk" is a TLD. mozilla::pkix does
        // not know that.
        (b"*.co.uk", b"example.co.uk", Ok(true)),
        (b"*.com", b"foo.com", Err(Error::MalformedDnsIdentifier)),
        (b"*.us", b"foo.us", Err(Error::MalformedDnsIdentifier)),
        (b"*", b"foo", Err(Error::MalformedDnsIdentifier)),
        // IDN variants of wildcards and registry controlled domains.
        (
            b"*.xn--poema-9qae5a.com.br",
            b"www.xn--poema-9qae5a.com.br",
            Ok(true),
        ),
        (
            b"*.example.xn--mgbaam7a8h",
            b"test.example.xn--mgbaam7a8h",
            Ok(true),
        ),
        // RFC6126 allows this, and NSS accepts it, but Chromium disallows it.
        // TODO: File bug against Chromium.
        (b"*.com.br", b"xn--poema-9qae5a.com.br", Ok(true)),
        (
            b"*.xn--mgbaam7a8h",
            b"example.xn--mgbaam7a8h",
            Err(Error::MalformedDnsIdentifier),
        ),
        // Wildcards should be permissible for 'private' registry-controlled
        // domains. (In mozilla::pkix, we do not know if it is a private registry-
        // controlled domain or not.)
        (b"*.appspot.com", b"www.appspot.com", Ok(true)),
        (b"*.s3.amazonaws.com", b"foo.s3.amazonaws.com", Ok(true)),
        // Multiple wildcards are not valid.
        (
            b"*.*.com",
            b"foo.example.com",
            Err(Error::MalformedDnsIdentifier),
        ),
        (
            b"*.bar.*.com",
            b"foo.bar.example.com",
            Err(Error::MalformedDnsIdentifier),
        ),
        // Absolute vs relative DNS name tests. Although not explicitly specified
        // in RFC 6125, absolute reference names (those ending in a .) should
        // match either absolute or relative presented names.
        // TODO: File errata against RFC 6125 about this.
        (b"foo.com.", b"foo.com", Err(Error::MalformedDnsIdentifier)),
        (b"foo.com", b"foo.com.", Ok(true)),
        (b"foo.com.", b"foo.com.", Err(Error::MalformedDnsIdentifier)),
        (b"f.", b"f", Err(Error::MalformedDnsIdentifier)),
        (b"f", b"f.", Ok(true)),
        (b"f.", b"f.", Err(Error::MalformedDnsIdentifier)),
        (
            b"*.bar.foo.com.",
            b"www-3.bar.foo.com",
            Err(Error::MalformedDnsIdentifier),
        ),
        (b"*.bar.foo.com", b"www-3.bar.foo.com.", Ok(true)),
        (
            b"*.bar.foo.com.",
            b"www-3.bar.foo.com.",
            Err(Error::MalformedDnsIdentifier),
        ),
        // We require the reference ID to be a valid DNS name, so we cannot test this
        // case.
        //     (b".", b".", Ok(false)),
        (
            b"*.com.",
            b"example.com",
            Err(Error::MalformedDnsIdentifier),
        ),
        (
            b"*.com",
            b"example.com.",
            Err(Error::MalformedDnsIdentifier),
        ),
        (
            b"*.com.",
            b"example.com.",
            Err(Error::MalformedDnsIdentifier),
        ),
        (b"*.", b"foo.", Err(Error::MalformedDnsIdentifier)),
        (b"*.", b"foo", Err(Error::MalformedDnsIdentifier)),
        // The result is different than Chromium because we don't know that co.uk is
        // a TLD.
        (
            b"*.co.uk.",
            b"foo.co.uk",
            Err(Error::MalformedDnsIdentifier),
        ),
        (
            b"*.co.uk.",
            b"foo.co.uk.",
            Err(Error::MalformedDnsIdentifier),
        ),
    ];

    #[test]
    fn presented_matches_reference_test() {
        for (presented, reference, expected_result) in PRESENTED_MATCHES_REFERENCE {
            let actual_result = presented_id_matches_reference_id(
                untrusted::Input::from(presented),
                IdRole::Reference,
                untrusted::Input::from(reference),
            );
            assert_eq!(
                &actual_result, expected_result,
                "presented_id_matches_reference_id(\"{presented:?}\", \"{reference:?}\")"
            );
        }
    }

    // (presented_name, constraint, expected_matches)
    #[allow(clippy::type_complexity)]
    const PRESENTED_MATCHES_CONSTRAINT: &[(&[u8], &[u8], Result<bool, Error>)] = &[
        // No absolute presented IDs allowed
        (b".", b"", Err(Error::MalformedDnsIdentifier)),
        (b"www.example.com.", b"", Err(Error::MalformedDnsIdentifier)),
        (
            b"www.example.com.",
            b"www.example.com.",
            Err(Error::MalformedDnsIdentifier),
        ),
        // No absolute constraints allowed
        (
            b"www.example.com",
            b".",
            Err(Error::MalformedNameConstraint),
        ),
        (
            b"www.example.com",
            b"www.example.com.",
            Err(Error::MalformedNameConstraint),
        ),
        // No wildcard in constraints allowed
        (
            b"www.example.com",
            b"*.example.com",
            Err(Error::MalformedNameConstraint),
        ),
        // No empty presented IDs allowed
        (b"", b"", Err(Error::MalformedDnsIdentifier)),
        // Empty constraints match everything allowed
        (b"example.com", b"", Ok(true)),
        (b"*.example.com", b"", Ok(true)),
        // Constraints that start with a dot
        (b"www.example.com", b".example.com", Ok(true)),
        (b"www.example.com", b".EXAMPLE.COM", Ok(true)),
        (b"www.example.com", b".axample.com", Ok(false)),
        (b"www.example.com", b".xample.com", Ok(false)),
        (b"www.example.com", b".exampl.com", Ok(false)),
        (b"badexample.com", b".example.com", Ok(false)),
        // Constraints that do not start with a dot
        (b"www.example.com", b"example.com", Ok(true)),
        (b"www.example.com", b"EXAMPLE.COM", Ok(true)),
        (b"www.example.com", b"axample.com", Ok(false)),
        (b"www.example.com", b"xample.com", Ok(false)),
        (b"www.example.com", b"exampl.com", Ok(false)),
        (b"badexample.com", b"example.com", Ok(false)),
        // Presented IDs with wildcard
        (b"*.example.com", b".example.com", Ok(true)),
        (b"*.example.com", b"example.com", Ok(true)),
        // `*.example.com` expands to names like `evil.example.com` that are
        // outside the subtree `www.example.com`, so it is not contained.
        (b"*.example.com", b"www.example.com", Ok(false)),
        (b"*.example.com", b"www.EXAMPLE.COM", Ok(false)),
        (b"*.example.com", b"www.axample.com", Ok(false)),
        (b"*.example.com", b".xample.com", Ok(false)),
        (b"*.example.com", b"xample.com", Ok(false)),
        (b"*.example.com", b".exampl.com", Ok(false)),
        (b"*.example.com", b"exampl.com", Ok(false)),
        // Matching IDs
        (b"www.example.com", b"www.example.com", Ok(true)),
    ];

    #[test]
    fn presented_matches_constraint_test() {
        for (presented, constraint, expected_result) in PRESENTED_MATCHES_CONSTRAINT {
            let actual_result = presented_id_matches_reference_id(
                untrusted::Input::from(presented),
                IdRole::NameConstraint(Subtrees::Permitted),
                untrusted::Input::from(constraint),
            );
            assert_eq!(
                &actual_result, expected_result,
                "presented_id_matches_constraint(\"{presented:?}\", \"{constraint:?}\")",
            );
        }
    }

    #[test]
    fn wildcard_san_not_contained_in_constraint() {
        for (presented, constraint, expected_result) in WILDCARD_CONSTRAINT_CONTAINMENT {
            let actual_result = presented_id_matches_reference_id(
                untrusted::Input::from(presented),
                IdRole::NameConstraint(Subtrees::Permitted),
                untrusted::Input::from(constraint),
            );
            assert_eq!(
                &actual_result, expected_result,
                "presented_id_matches_constraint(\"{presented:?}\", \"{constraint:?}\")",
            );
        }
    }

    // Per RFC 5280 4.2.1.10, a permitted dNSName subtree `www.example.com`
    // covers only names formed by prepending labels to `www.example.com`.
    // A wildcard SAN `*.example.com` can expand to e.g. `evil.example.com`,
    // which is outside that subtree, so it must not be considered contained.
    // In contrast, `*.www.example.com` only expands to names inside the
    // subtree and is correctly accepted.
    #[expect(clippy::type_complexity)]
    const WILDCARD_CONSTRAINT_CONTAINMENT: &[(&[u8], &[u8], Result<bool, Error>)] = &[
        // Bug: `*.example.com` is broader than the permitted subtree
        // `www.example.com` and must not satisfy the constraint.
        (b"*.example.com", b"www.example.com", Ok(false)),
        // Control: `*.www.example.com` stays within the subtree.
        (b"*.www.example.com", b"www.example.com", Ok(true)),
        // Further out-of-subtree wildcard SANs that must be rejected.
        (b"*.example.com", b"a.b.example.com", Ok(false)),
        (b"*.b.example.com", b"a.b.example.com", Ok(false)),
    ];

    // For excluded subtrees, a wildcard SAN must be treated as matching if
    // any of its expansions could fall inside the excluded subtree
    // (CVE-2025-61727). This is the opposite polarity from the containment
    // test used for permitted subtrees.
    #[test]
    fn wildcard_san_could_match_excluded_subtree() {
        for (presented, constraint, expected_result) in WILDCARD_EXCLUDED_INTERSECTION {
            let actual_result = presented_id_matches_reference_id(
                untrusted::Input::from(presented),
                IdRole::NameConstraint(Subtrees::Excluded),
                untrusted::Input::from(constraint),
            );
            assert_eq!(
                &actual_result, expected_result,
                "presented_id_matches_constraint(\"{presented:?}\", \"{constraint:?}\")",
            );
        }
    }

    #[expect(clippy::type_complexity)]
    const WILDCARD_EXCLUDED_INTERSECTION: &[(&[u8], &[u8], Result<bool, Error>)] = &[
        // All expansions of `*.example.com` fall under the excluded subtree.
        (b"*.example.com", b"example.com", Ok(true)),
        (b"*.example.com", b".example.com", Ok(true)),
        // `*.example.com` can expand to `www.example.com`, which is inside
        // the excluded subtree rooted at `www.example.com`.
        (b"*.example.com", b"www.example.com", Ok(true)),
        (b"*.example.com", b"www.EXAMPLE.COM", Ok(true)),
        // The wildcard cannot reach two labels deep, so it does not
        // intersect a more specific excluded subtree.
        (b"*.example.com", b"a.b.example.com", Ok(false)),
        // Disjoint parent labels never intersect.
        (b"*.example.com", b"www.other.com", Ok(false)),
    ];
}
