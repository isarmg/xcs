#[cfg(all(test, feature = "std"))]
mod alloc_tests {
    use super::*;

    // (presented_address, constraint_address, constraint_mask, expected_result)
    const PRESENTED_MATCHES_CONSTRAINT: &[(&str, &str, &str, Result<bool, Error>)] = &[
        // Cannot mix IpV4 with IpV6 and viceversa
        ("2001:db8::", "8.8.8.8", "255.255.255.255", Ok(false)),
        ("8.8.8.8", "2001:db8::", "ffff::", Ok(false)),
        // IpV4 non-contiguous masks
        (
            "8.8.8.8",
            "8.8.8.8",
            "255.255.255.1",
            Err(Error::InvalidNetworkMaskConstraint),
        ),
        (
            "8.8.8.8",
            "8.8.8.8",
            "255.255.0.255",
            Err(Error::InvalidNetworkMaskConstraint),
        ),
        (
            "8.8.8.8",
            "8.8.8.8",
            "255.0.255.255",
            Err(Error::InvalidNetworkMaskConstraint),
        ),
        (
            "8.8.8.8",
            "8.8.8.8",
            "0.255.255.255",
            Err(Error::InvalidNetworkMaskConstraint),
        ),
        (
            "8.8.8.8",
            "8.8.8.8",
            "1.255.255.255",
            Err(Error::InvalidNetworkMaskConstraint),
        ),
        (
            "8.8.8.8",
            "8.8.8.8",
            "128.128.128.128",
            Err(Error::InvalidNetworkMaskConstraint),
        ),
        // IpV4
        ("8.8.8.8", "8.8.8.8", "255.255.255.255", Ok(true)),
        ("8.8.8.9", "8.8.8.8", "255.255.255.255", Ok(false)),
        ("8.8.8.9", "8.8.8.8", "255.255.255.254", Ok(true)),
        ("8.8.8.10", "8.8.8.8", "255.255.255.254", Ok(false)),
        ("8.8.8.10", "8.8.8.8", "255.255.255.0", Ok(true)),
        ("8.8.15.10", "8.8.8.8", "255.255.248.0", Ok(true)),
        ("8.8.16.10", "8.8.8.8", "255.255.248.0", Ok(false)),
        ("8.8.16.10", "8.8.8.8", "255.255.0.0", Ok(true)),
        ("8.31.16.10", "8.8.8.8", "255.224.0.0", Ok(true)),
        ("8.32.16.10", "8.8.8.8", "255.224.0.0", Ok(false)),
        ("8.32.16.10", "8.8.8.8", "255.0.0.0", Ok(true)),
        ("63.32.16.10", "8.8.8.8", "192.0.0.0", Ok(true)),
        ("64.32.16.10", "8.8.8.8", "192.0.0.0", Ok(false)),
        ("64.32.16.10", "8.8.8.8", "0.0.0.0", Ok(true)),
        // IpV6 non-contiguous masks
        (
            "2001:db8::",
            "2001:db8::",
            "fffe:ffff::",
            Err(Error::InvalidNetworkMaskConstraint),
        ),
        (
            "2001:db8::",
            "2001:db8::",
            "ffff:fdff::",
            Err(Error::InvalidNetworkMaskConstraint),
        ),
        (
            "2001:db8::",
            "2001:db8::",
            "ffff:feff::",
            Err(Error::InvalidNetworkMaskConstraint),
        ),
        (
            "2001:db8::",
            "2001:db8::",
            "ffff:fcff::",
            Err(Error::InvalidNetworkMaskConstraint),
        ),
        (
            "2001:db8::",
            "2001:db8::",
            "7fff:ffff::",
            Err(Error::InvalidNetworkMaskConstraint),
        ),
        // IpV6
        ("2001:db8::", "2001:db8::", "ffff:ffff::", Ok(true)),
        ("2001:db9::", "2001:db8::", "ffff:ffff::", Ok(false)),
        ("2001:db9::", "2001:db8::", "ffff:fffe::", Ok(true)),
        ("2001:dba::", "2001:db8::", "ffff:fffe::", Ok(false)),
        ("2001:dba::", "2001:db8::", "ffff:ff00::", Ok(true)),
        ("2001:dca::", "2001:db8::", "ffff:fe00::", Ok(true)),
        ("2001:fca::", "2001:db8::", "ffff:fe00::", Ok(false)),
        ("2001:fca::", "2001:db8::", "ffff:0000::", Ok(true)),
        ("2000:fca::", "2001:db8::", "fffe:0000::", Ok(true)),
        ("2003:fca::", "2001:db8::", "fffe:0000::", Ok(false)),
        ("2003:fca::", "2001:db8::", "ff00:0000::", Ok(true)),
        ("1003:fca::", "2001:db8::", "e000:0000::", Ok(false)),
        ("1003:fca::", "2001:db8::", "0000:0000::", Ok(true)),
    ];

    #[test]
    fn presented_matches_constraint_test() {
        use std::boxed::Box;
        use std::net::IpAddr;

        for (presented, constraint_address, constraint_mask, expected_result) in
            PRESENTED_MATCHES_CONSTRAINT
        {
            let presented_bytes: Box<[u8]> = match presented.parse::<IpAddr>().unwrap() {
                IpAddr::V4(p) => Box::new(p.octets()),
                IpAddr::V6(p) => Box::new(p.octets()),
            };
            let ca_bytes: Box<[u8]> = match constraint_address.parse::<IpAddr>().unwrap() {
                IpAddr::V4(ca) => Box::new(ca.octets()),
                IpAddr::V6(ca) => Box::new(ca.octets()),
            };
            let cm_bytes: Box<[u8]> = match constraint_mask.parse::<IpAddr>().unwrap() {
                IpAddr::V4(cm) => Box::new(cm.octets()),
                IpAddr::V6(cm) => Box::new(cm.octets()),
            };
            let constraint_bytes = [ca_bytes, cm_bytes].concat();
            let actual_result = presented_id_matches_constraint(
                untrusted::Input::from(&presented_bytes),
                untrusted::Input::from(&constraint_bytes),
            );
            assert_eq!(
                &actual_result, expected_result,
                "presented_id_matches_constraint(\"{presented_bytes:?}\", \"{constraint_bytes:?}\")"
            );
        }
    }
}
