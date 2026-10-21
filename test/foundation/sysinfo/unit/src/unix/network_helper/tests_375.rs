#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipv4_mask() {
        let mask = Ipv4Addr::new(255, 255, 255, 0);
        let prefix = ipv4_mask_to_prefix(mask).unwrap();
        assert_eq!(prefix, 24);
    }

    #[test]
    fn ipv4_mask_another() {
        let mask = Ipv4Addr::new(255, 255, 255, 128);
        let prefix = ipv4_mask_to_prefix(mask).unwrap();
        assert_eq!(prefix, 25);
    }

    #[test]
    fn v4_mask_to_prefix_invalid() {
        let mask = Ipv4Addr::new(255, 128, 255, 0);
        assert!(ipv4_mask_to_prefix(mask).is_err());
    }

    #[test]
    fn ipv6_mask() {
        let mask = Ipv6Addr::new(0xffff, 0xffff, 0xffff, 0, 0, 0, 0, 0);
        let prefix = ipv6_mask_to_prefix(mask).unwrap();
        assert_eq!(prefix, 48);
    }

    #[test]
    fn ipv6_mask_invalid() {
        let mask = Ipv6Addr::new(0, 0xffff, 0xffff, 0, 0, 0, 0, 0);
        assert!(ipv6_mask_to_prefix(mask).is_err());
    }

    #[test]
    fn ip_mask_enum_ipv4() {
        let mask = IpAddr::from(Ipv4Addr::new(255, 255, 255, 0));
        let prefix = ip_mask_to_prefix(mask).unwrap();
        assert_eq!(prefix, 24);
    }

    #[test]
    fn ip_mask_enum_ipv4_invalid() {
        let mask = IpAddr::from(Ipv4Addr::new(255, 0, 255, 0));
        assert!(ip_mask_to_prefix(mask).is_err());
    }

    #[test]
    fn ip_mask_enum_ipv6() {
        let mask = IpAddr::from(Ipv6Addr::new(0xffff, 0xffff, 0xffff, 0, 0, 0, 0, 0));
        let prefix = ip_mask_to_prefix(mask).unwrap();
        assert_eq!(prefix, 48);
    }

    #[test]
    fn ip_mask_enum_ipv6_invalid() {
        let mask = IpAddr::from(Ipv6Addr::new(0xffff, 0xffff, 0xff00, 0xffff, 0, 0, 0, 0));
        assert!(ip_mask_to_prefix(mask).is_err());
    }
}
