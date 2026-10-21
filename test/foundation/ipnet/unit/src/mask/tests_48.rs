#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Ipv4Net, Ipv6Net};

    #[test]
    fn v4_mask_to_prefix() {
        let mask = Ipv4Addr::new(255, 255, 255, 128);
        let prefix = ipv4_mask_to_prefix(mask);
        assert_eq!(prefix, Ok(25));
    }

    #[test]
    fn v4_mask_to_prefix_max() {
        let mask = Ipv4Addr::from(u32::MAX);
        let prefix = ipv4_mask_to_prefix(mask);
        assert_eq!(prefix, Ok(32));
    }

    #[test]
    fn invalid_v4_mask_to_prefix() {
        let mask = Ipv4Addr::new(255, 0, 255, 0);
        let prefix = ipv4_mask_to_prefix(mask);
        assert!(prefix.is_err());
    }

    #[test]
    fn ipv4net_with_netmask() {
        {
            // Positive test-case.
            let addr = Ipv4Addr::new(127, 0, 0, 1);
            let mask = Ipv4Addr::new(255, 0, 0, 0);
            let net = Ipv4Net::with_netmask(addr, mask).unwrap();
            let expected = Ipv4Net::new(Ipv4Addr::new(127, 0, 0, 1), 8).unwrap();
            assert_eq!(net, expected);
        }
        {
            // Negative test-case.
            let addr = Ipv4Addr::new(127, 0, 0, 1);
            let mask = Ipv4Addr::new(255, 0, 255, 0);
            Ipv4Net::with_netmask(addr, mask).unwrap_err();
        }
    }

    #[test]
    fn v6_mask_to_prefix() {
        let mask = Ipv6Addr::new(0xffff, 0xffff, 0xffff, 0, 0, 0, 0, 0);
        let prefix = ipv6_mask_to_prefix(mask);
        assert_eq!(prefix, Ok(48));
    }

    #[test]
    fn v6_mask_to_prefix_max() {
        let mask = Ipv6Addr::from(u128::MAX);
        let prefix = ipv6_mask_to_prefix(mask);
        assert_eq!(prefix, Ok(128));
    }

    #[test]
    fn invalid_v6_mask_to_prefix() {
        let mask = Ipv6Addr::new(0, 0, 0xffff, 0xffff, 0, 0, 0, 0);
        let prefix = ipv6_mask_to_prefix(mask);
        assert!(prefix.is_err());
    }

    #[test]
    fn ipv6net_with_netmask() {
        {
            // Positive test-case.
            let addr = Ipv6Addr::new(0xff01, 0, 0, 0x17, 0, 0, 0, 0x2);
            let mask = Ipv6Addr::new(0xffff, 0xffff, 0xffff, 0, 0, 0, 0, 0);
            let net = Ipv6Net::with_netmask(addr, mask).unwrap();
            let expected =
                Ipv6Net::new(Ipv6Addr::new(0xff01, 0, 0, 0x17, 0, 0, 0, 0x2), 48).unwrap();
            assert_eq!(net, expected);
        }
        {
            // Negative test-case.
            let addr = Ipv6Addr::new(0xff01, 0, 0, 0x17, 0, 0, 0, 0x2);
            let mask = Ipv6Addr::new(0, 0, 0xffff, 0xffff, 0, 0, 0, 0);
            Ipv6Net::with_netmask(addr, mask).unwrap_err();
        }
    }
}
