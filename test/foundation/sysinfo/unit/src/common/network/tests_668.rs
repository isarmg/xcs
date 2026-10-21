#[cfg(test)]
mod tests {
    use crate::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use std::str::FromStr;

    // Ensure that the `Display` and `Debug` traits are implemented on the `MacAddr` struct
    #[test]
    fn check_display_impl_mac_address() {
        println!(
            "{} {:?}",
            MacAddr([0x1, 0x2, 0x3, 0x4, 0x5, 0x6]),
            MacAddr([0xa, 0xb, 0xc, 0xd, 0xe, 0xf])
        );
    }

    #[test]
    fn check_mac_address_is_unspecified_true() {
        assert!(MacAddr::UNSPECIFIED.is_unspecified());
        assert!(MacAddr([0; 6]).is_unspecified());
    }

    #[test]
    fn check_mac_address_is_unspecified_false() {
        assert!(!MacAddr([1, 2, 3, 4, 5, 6]).is_unspecified());
    }

    #[test]
    fn check_mac_address_conversions() {
        let mac = MacAddr([0xa, 0xb, 0xc, 0xd, 0xe, 0xf]);

        let mac_s = mac.to_string();
        assert_eq!("0a:0b:0c:0d:0e:0f", mac_s);
        assert_eq!(Ok(mac), MacAddr::from_str(&mac_s));

        assert_eq!(
            MacAddr::from_str("0a:0b:0c:0d:0e:0f:01"),
            Err(MacAddrFromStrError::InvalidAddrFormat)
        );
        assert_eq!(
            MacAddr::from_str("0a:0b:0c:0d:0e"),
            Err(MacAddrFromStrError::InvalidAddrFormat)
        );
    }

    // Ensure that the `Display` and `Debug` traits are implemented on the `IpNetwork` struct
    #[test]
    fn check_display_impl_ip_network_ipv4() {
        println!(
            "{} {:?}",
            IpNetwork {
                addr: IpAddr::from(Ipv4Addr::new(1, 2, 3, 4)),
                prefix: 3
            },
            IpNetwork {
                addr: IpAddr::from(Ipv4Addr::new(255, 255, 255, 0)),
                prefix: 21
            }
        );
    }

    #[test]
    fn check_display_impl_ip_network_ipv6() {
        println!(
            "{} {:?}",
            IpNetwork {
                addr: IpAddr::from(Ipv6Addr::new(0xffff, 0xaabb, 00, 0, 0, 0x000c, 11, 21)),
                prefix: 127
            },
            IpNetwork {
                addr: IpAddr::from(Ipv6Addr::new(0xffcc, 0, 0, 0xffcc, 0, 0xffff, 0, 0xccaa)),
                prefix: 120
            }
        )
    }

    #[test]
    fn check_ip_networks() {
        if !IS_SUPPORTED_SYSTEM {
            return;
        }
        let networks = Networks::new_with_refreshed_list();
        if networks.iter().any(|(_, n)| !n.ip_networks().is_empty()) {
            return;
        }
        panic!("Networks should have at least one IP network ");
    }

    #[test]
    fn check_ip_network_conversions() {
        let addr = IpNetwork {
            addr: IpAddr::from(Ipv6Addr::new(0xff, 0xa, 0x8, 0x12, 0x7, 0xc, 0xa, 0xb)),
            prefix: 12,
        };

        let addr_s = addr.to_string();
        assert_eq!("ff:a:8:12:7:c:a:b/12", addr_s);
        assert_eq!(Ok(addr), IpNetwork::from_str(&addr_s));

        let addr = IpNetwork {
            addr: IpAddr::from(Ipv4Addr::new(255, 255, 255, 0)),
            prefix: 21,
        };

        let addr_s = addr.to_string();
        assert_eq!("255.255.255.0/21", addr_s);
        assert_eq!(Ok(addr), IpNetwork::from_str(&addr_s));

        assert_eq!(
            IpNetwork::from_str("ff:a:8:12:7:c:a:b"),
            Err(IpNetworkFromStrError::InvalidAddrFormat)
        );
        assert_eq!(
            IpNetwork::from_str("ff:a:8:12:7:c:a:b/12/12"),
            Err(IpNetworkFromStrError::InvalidAddrFormat)
        );
        match IpNetwork::from_str("0a:0b:0c:0d:0e/12") {
            Err(IpNetworkFromStrError::AddrParseError(_)) => {}
            x => panic!("expected `IpNetworkFromStrError::AddrParseError`, found {x:?}"),
        }
    }
}
