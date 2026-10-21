#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "alloc")]
    use alloc::format;

    #[cfg(feature = "alloc")]
    static TESTS: &[(&str, bool)] = &[
        ("", false),
        ("localhost", true),
        ("LOCALHOST", true),
        (".localhost", false),
        ("..localhost", false),
        ("1.2.3.4", false),
        ("127.0.0.1", false),
        ("absolute.", true),
        ("absolute..", false),
        ("multiple.labels.absolute.", true),
        ("foo.bar.com", true),
        ("infix-hyphen-allowed.com", true),
        ("-prefixhypheninvalid.com", false),
        ("suffixhypheninvalid--", false),
        ("suffixhypheninvalid-.com", false),
        ("foo.lastlabelendswithhyphen-", false),
        ("infix_underscore_allowed.com", true),
        ("_prefixunderscorevalid.com", true),
        ("labelendswithnumber1.bar.com", true),
        ("xn--bcher-kva.example", true),
        (
            "sixtythreesixtythreesixtythreesixtythreesixtythreesixtythreesix.com",
            true,
        ),
        (
            "sixtyfoursixtyfoursixtyfoursixtyfoursixtyfoursixtyfoursixtyfours.com",
            false,
        ),
        (
            "012345678901234567890123456789012345678901234567890123456789012.com",
            true,
        ),
        (
            "0123456789012345678901234567890123456789012345678901234567890123.com",
            false,
        ),
        (
            "01234567890123456789012345678901234567890123456789012345678901-.com",
            false,
        ),
        (
            "012345678901234567890123456789012345678901234567890123456789012-.com",
            false,
        ),
        ("numeric-only-final-label.1", false),
        ("numeric-only-final-label.absolute.1.", false),
        ("1starts-with-number.com", true),
        ("1Starts-with-number.com", true),
        ("1.2.3.4.com", true),
        ("123.numeric-only-first-label", true),
        ("a123b.com", true),
        ("numeric-only-middle-label.4.com", true),
        ("1000-sans.badssl.com", true),
        (
            "twohundredandfiftythreecharacters.twohundredandfiftythreecharacters.twohundredandfiftythreecharacters.twohundredandfiftythreecharacters.twohundredandfiftythreecharacters.twohundredandfiftythreecharacters.twohundredandfiftythreecharacters.twohundredandfi",
            true,
        ),
        (
            "twohundredandfiftyfourcharacters.twohundredandfiftyfourcharacters.twohundredandfiftyfourcharacters.twohundredandfiftyfourcharacters.twohundredandfiftyfourcharacters.twohundredandfiftyfourcharacters.twohundredandfiftyfourcharacters.twohundredandfiftyfourc",
            false,
        ),
    ];

    #[cfg(feature = "alloc")]
    #[test]
    fn test_validation() {
        for (input, expected) in TESTS {
            #[cfg(feature = "std")]
            println!("test: {input:?} expected valid? {expected:?}");
            let name_ref = DnsName::try_from(*input);
            assert_eq!(*expected, name_ref.is_ok());
            let name = DnsName::try_from(input.to_string());
            assert_eq!(*expected, name.is_ok());
        }
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn error_is_debug() {
        assert_eq!(format!("{InvalidDnsNameError:?}"), "InvalidDnsNameError");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn error_is_display() {
        assert_eq!(format!("{InvalidDnsNameError}"), "invalid dns name");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn dns_name_is_debug() {
        let example = DnsName::try_from("example.com".to_string()).unwrap();
        assert_eq!(format!("{example:?}"), "DnsName(\"example.com\")");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn dns_name_traits() {
        let example = DnsName::try_from("example.com".to_string()).unwrap();
        assert_eq!(example, example); // PartialEq

        #[cfg(feature = "std")]
        {
            use std::collections::HashSet;
            let mut h = HashSet::<DnsName<'_>>::new();
            h.insert(example);
        }
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn try_from_ascii_rejects_bad_utf8() {
        assert_eq!(
            format!("{:?}", DnsName::try_from(&b"\x80"[..])),
            "Err(InvalidDnsNameError)"
        );
    }

    const fn ipv4_address(
        ip_address: &str,
        octets: [u8; 4],
    ) -> (&str, Result<Ipv4Addr, AddrParseError>) {
        (ip_address, Ok(Ipv4Addr(octets)))
    }

    const IPV4_ADDRESSES: &[(&str, Result<Ipv4Addr, AddrParseError>)] = &[
        // Valid IPv4 addresses
        ipv4_address("0.0.0.0", [0, 0, 0, 0]),
        ipv4_address("1.1.1.1", [1, 1, 1, 1]),
        ipv4_address("205.0.0.0", [205, 0, 0, 0]),
        ipv4_address("0.205.0.0", [0, 205, 0, 0]),
        ipv4_address("0.0.205.0", [0, 0, 205, 0]),
        ipv4_address("0.0.0.205", [0, 0, 0, 205]),
        ipv4_address("0.0.0.20", [0, 0, 0, 20]),
        // Invalid IPv4 addresses
        ("", Err(AddrParseError(AddrKind::Ipv4))),
        ("...", Err(AddrParseError(AddrKind::Ipv4))),
        (".0.0.0.0", Err(AddrParseError(AddrKind::Ipv4))),
        ("0.0.0.0.", Err(AddrParseError(AddrKind::Ipv4))),
        ("0.0.0", Err(AddrParseError(AddrKind::Ipv4))),
        ("0.0.0.", Err(AddrParseError(AddrKind::Ipv4))),
        ("256.0.0.0", Err(AddrParseError(AddrKind::Ipv4))),
        ("0.256.0.0", Err(AddrParseError(AddrKind::Ipv4))),
        ("0.0.256.0", Err(AddrParseError(AddrKind::Ipv4))),
        ("0.0.0.256", Err(AddrParseError(AddrKind::Ipv4))),
        ("1..1.1.1", Err(AddrParseError(AddrKind::Ipv4))),
        ("1.1..1.1", Err(AddrParseError(AddrKind::Ipv4))),
        ("1.1.1..1", Err(AddrParseError(AddrKind::Ipv4))),
        ("025.0.0.0", Err(AddrParseError(AddrKind::Ipv4))),
        ("0.025.0.0", Err(AddrParseError(AddrKind::Ipv4))),
        ("0.0.025.0", Err(AddrParseError(AddrKind::Ipv4))),
        ("0.0.0.025", Err(AddrParseError(AddrKind::Ipv4))),
        ("1234.0.0.0", Err(AddrParseError(AddrKind::Ipv4))),
        ("0.1234.0.0", Err(AddrParseError(AddrKind::Ipv4))),
        ("0.0.1234.0", Err(AddrParseError(AddrKind::Ipv4))),
        ("0.0.0.1234", Err(AddrParseError(AddrKind::Ipv4))),
    ];

    #[test]
    fn parse_ipv4_address_test() {
        for &(ip_address, expected_result) in IPV4_ADDRESSES {
            assert_eq!(Ipv4Addr::try_from(ip_address), expected_result);
        }
    }

    const fn ipv6_address(
        ip_address: &str,
        octets: [u8; 16],
    ) -> (&str, Result<Ipv6Addr, AddrParseError>) {
        (ip_address, Ok(Ipv6Addr(octets)))
    }

    const IPV6_ADDRESSES: &[(&str, Result<Ipv6Addr, AddrParseError>)] = &[
        // Valid IPv6 addresses
        ipv6_address(
            "2a05:d018:076c:b685:e8ab:afd3:af51:3aed",
            [
                0x2a, 0x05, 0xd0, 0x18, 0x07, 0x6c, 0xb6, 0x85, 0xe8, 0xab, 0xaf, 0xd3, 0xaf, 0x51,
                0x3a, 0xed,
            ],
        ),
        ipv6_address(
            "2A05:D018:076C:B685:E8AB:AFD3:AF51:3AED",
            [
                0x2a, 0x05, 0xd0, 0x18, 0x07, 0x6c, 0xb6, 0x85, 0xe8, 0xab, 0xaf, 0xd3, 0xaf, 0x51,
                0x3a, 0xed,
            ],
        ),
        ipv6_address(
            "ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff",
            [
                0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                0xff, 0xff,
            ],
        ),
        ipv6_address(
            "FFFF:FFFF:FFFF:FFFF:FFFF:FFFF:FFFF:FFFF",
            [
                0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                0xff, 0xff,
            ],
        ),
        ipv6_address(
            "FFFF:ffff:ffff:ffff:ffff:ffff:ffff:ffff",
            [
                0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                0xff, 0xff,
            ],
        ),
        // Wrong hexadecimal characters on different positions
        (
            "ffgf:ffff:ffff:ffff:ffff:ffff:ffff:ffff",
            Err(AddrParseError(AddrKind::Ipv6)),
        ),
        (
            "ffff:gfff:ffff:ffff:ffff:ffff:ffff:ffff",
            Err(AddrParseError(AddrKind::Ipv6)),
        ),
        (
            "ffff:ffff:fffg:ffff:ffff:ffff:ffff:ffff",
            Err(AddrParseError(AddrKind::Ipv6)),
        ),
        (
            "ffff:ffff:ffff:ffgf:ffff:ffff:ffff:ffff",
            Err(AddrParseError(AddrKind::Ipv6)),
        ),
        (
            "ffff:ffff:ffff:ffff:gfff:ffff:ffff:ffff",
            Err(AddrParseError(AddrKind::Ipv6)),
        ),
        (
            "ffff:ffff:ffff:ffff:ffff:fgff:ffff:ffff",
            Err(AddrParseError(AddrKind::Ipv6)),
        ),
        (
            "ffff:ffff:ffff:ffff:ffff:ffff:ffgf:ffff",
            Err(AddrParseError(AddrKind::Ipv6)),
        ),
        (
            "ffff:ffff:ffff:ffff:ffff:ffff:ffgf:fffg",
            Err(AddrParseError(AddrKind::Ipv6)),
        ),
        // Wrong colons on uncompressed addresses
        (
            ":ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff",
            Err(AddrParseError(AddrKind::Ipv6)),
        ),
        (
            "ffff::ffff:ffff:ffff:ffff:ffff:ffff:ffff",
            Err(AddrParseError(AddrKind::Ipv6)),
        ),
        (
            "ffff:ffff::ffff:ffff:ffff:ffff:ffff:ffff",
            Err(AddrParseError(AddrKind::Ipv6)),
        ),
        (
            "ffff:ffff:ffff::ffff:ffff:ffff:ffff:ffff",
            Err(AddrParseError(AddrKind::Ipv6)),
        ),
        (
            "ffff:ffff:ffff:ffff::ffff:ffff:ffff:ffff",
            Err(AddrParseError(AddrKind::Ipv6)),
        ),
        (
            "ffff:ffff:ffff:ffff:ffff::ffff:ffff:ffff",
            Err(AddrParseError(AddrKind::Ipv6)),
        ),
        (
            "ffff:ffff:ffff:ffff:ffff:ffff::ffff:ffff",
            Err(AddrParseError(AddrKind::Ipv6)),
        ),
        (
            "ffff:ffff:ffff:ffff:ffff:ffff:ffff::ffff",
            Err(AddrParseError(AddrKind::Ipv6)),
        ),
        // More colons than allowed
        (
            "ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff:",
            Err(AddrParseError(AddrKind::Ipv6)),
        ),
        (
            "ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff",
            Err(AddrParseError(AddrKind::Ipv6)),
        ),
        // v Invalid hexadecimal
        (
            "ga05:d018:076c:b685:e8ab:afd3:af51:3aed",
            Err(AddrParseError(AddrKind::Ipv6)),
        ),
        // Cannot start with colon
        (
            ":a05:d018:076c:b685:e8ab:afd3:af51:3aed",
            Err(AddrParseError(AddrKind::Ipv6)),
        ),
        // Cannot end with colon
        (
            "2a05:d018:076c:b685:e8ab:afd3:af51:3ae:",
            Err(AddrParseError(AddrKind::Ipv6)),
        ),
        // Cannot have more than seven colons
        (
            "2a05:d018:076c:b685:e8ab:afd3:af51:3a::",
            Err(AddrParseError(AddrKind::Ipv6)),
        ),
        // Cannot contain two colons in a row
        (
            "2a05::018:076c:b685:e8ab:afd3:af51:3aed",
            Err(AddrParseError(AddrKind::Ipv6)),
        ),
        // v Textual block size is longer
        (
            "2a056:d018:076c:b685:e8ab:afd3:af51:3ae",
            Err(AddrParseError(AddrKind::Ipv6)),
        ),
        // v Textual block size is shorter
        (
            "2a0:d018:076c:b685:e8ab:afd3:af51:3aed ",
            Err(AddrParseError(AddrKind::Ipv6)),
        ),
        // Shorter IPv6 address
        (
            "d018:076c:b685:e8ab:afd3:af51:3aed",
            Err(AddrParseError(AddrKind::Ipv6)),
        ),
        // Longer IPv6 address
        (
            "2a05:d018:076c:b685:e8ab:afd3:af51:3aed3aed",
            Err(AddrParseError(AddrKind::Ipv6)),
        ),
    ];

    #[test]
    fn parse_ipv6_address_test() {
        for &(ip_address, expected_result) in IPV6_ADDRESSES {
            assert_eq!(Ipv6Addr::try_from(ip_address), expected_result);
        }
    }

    #[test]
    fn try_from_ascii_ip_address_test() {
        const IP_ADDRESSES: &[(&str, Result<IpAddr, AddrParseError>)] = &[
            // Valid IPv4 addresses
            ("127.0.0.1", Ok(IpAddr::V4(Ipv4Addr([127, 0, 0, 1])))),
            // Invalid IPv4 addresses
            (
                // Ends with a dot; misses one octet
                "127.0.0.",
                Err(AddrParseError(AddrKind::Ipv6)),
            ),
            // Valid IPv6 addresses
            (
                "0000:0000:0000:0000:0000:0000:0000:0001",
                Ok(IpAddr::V6(Ipv6Addr([
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
                ]))),
            ),
            // Something else
            (
                // A hostname
                "example.com",
                Err(AddrParseError(AddrKind::Ipv6)),
            ),
        ];
        for &(ip_address, expected_result) in IP_ADDRESSES {
            assert_eq!(IpAddr::try_from(ip_address), expected_result)
        }
    }

    #[test]
    fn try_from_ascii_str_ip_address_test() {
        const IP_ADDRESSES: &[(&str, Result<IpAddr, AddrParseError>)] = &[
            // Valid IPv4 addresses
            ("127.0.0.1", Ok(IpAddr::V4(Ipv4Addr([127, 0, 0, 1])))),
            // Invalid IPv4 addresses
            (
                // Ends with a dot; misses one octet
                "127.0.0.",
                Err(AddrParseError(AddrKind::Ipv6)),
            ),
            // Valid IPv6 addresses
            (
                "0000:0000:0000:0000:0000:0000:0000:0001",
                Ok(IpAddr::V6(Ipv6Addr([
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
                ]))),
            ),
            // Something else
            (
                // A hostname
                "example.com",
                Err(AddrParseError(AddrKind::Ipv6)),
            ),
        ];
        for &(ip_address, expected_result) in IP_ADDRESSES {
            assert_eq!(IpAddr::try_from(ip_address), expected_result)
        }
    }

    #[test]
    #[cfg(feature = "std")]
    fn to_str() {
        let domain_str = "example.com";
        let domain_servername = ServerName::try_from(domain_str).unwrap();
        assert_eq!(domain_str, domain_servername.to_str());

        let ipv4_str = "127.0.0.1";
        let ipv4_servername = ServerName::try_from("127.0.0.1").unwrap();
        assert_eq!(ipv4_str, ipv4_servername.to_str());

        let ipv6_str = "::1";
        let ipv6_servername = ServerName::try_from(ipv6_str).unwrap();
        assert_eq!("::1", ipv6_servername.to_str());
    }
}
