#[cfg(test)]
mod tests {
    use alloc::vec::Vec;
    use core::str::FromStr;
    #[cfg(not(feature = "std"))]
    use core::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    #[cfg(feature = "std")]
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use super::*;

    #[test]
    fn test_ipaddrrange() {
        // Next, Next-Back
        let i = Ipv4AddrRange::new(
            Ipv4Addr::from_str("10.0.0.0").unwrap(),
            Ipv4Addr::from_str("10.0.0.3").unwrap()
        );

        assert_eq!(i.collect::<Vec<Ipv4Addr>>(), vec![
            Ipv4Addr::from_str("10.0.0.0").unwrap(),
            Ipv4Addr::from_str("10.0.0.1").unwrap(),
            Ipv4Addr::from_str("10.0.0.2").unwrap(),
            Ipv4Addr::from_str("10.0.0.3").unwrap(),
        ]);

        let mut v = i.collect::<Vec<_>>();
        v.reverse();
        assert_eq!(v, i.rev().collect::<Vec<_>>());

        let i = Ipv4AddrRange::new(
            Ipv4Addr::from_str("255.255.255.254").unwrap(),
            Ipv4Addr::from_str("255.255.255.255").unwrap()
        );

        assert_eq!(i.collect::<Vec<Ipv4Addr>>(), vec![
            Ipv4Addr::from_str("255.255.255.254").unwrap(),
            Ipv4Addr::from_str("255.255.255.255").unwrap(),
        ]);

        let i = Ipv6AddrRange::new(
            Ipv6Addr::from_str("fd00::").unwrap(),
            Ipv6Addr::from_str("fd00::3").unwrap(),
        );

        assert_eq!(i.collect::<Vec<Ipv6Addr>>(), vec![
            Ipv6Addr::from_str("fd00::").unwrap(),
            Ipv6Addr::from_str("fd00::1").unwrap(),
            Ipv6Addr::from_str("fd00::2").unwrap(),
            Ipv6Addr::from_str("fd00::3").unwrap(),
        ]);

        let mut v = i.collect::<Vec<_>>();
        v.reverse();
        assert_eq!(v, i.rev().collect::<Vec<_>>());

        let i = Ipv6AddrRange::new(
            Ipv6Addr::from_str("ffff:ffff:ffff:ffff:ffff:ffff:ffff:fffe").unwrap(),
            Ipv6Addr::from_str("ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff").unwrap(),
        );

        assert_eq!(i.collect::<Vec<Ipv6Addr>>(), vec![
            Ipv6Addr::from_str("ffff:ffff:ffff:ffff:ffff:ffff:ffff:fffe").unwrap(),
            Ipv6Addr::from_str("ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff").unwrap(),
        ]);

        let i = IpAddrRange::from(Ipv4AddrRange::new(
            Ipv4Addr::from_str("10.0.0.0").unwrap(),
            Ipv4Addr::from_str("10.0.0.3").unwrap(),
        ));

        assert_eq!(i.collect::<Vec<IpAddr>>(), vec![
            IpAddr::from_str("10.0.0.0").unwrap(),
            IpAddr::from_str("10.0.0.1").unwrap(),
            IpAddr::from_str("10.0.0.2").unwrap(),
            IpAddr::from_str("10.0.0.3").unwrap(),
        ]);

        let mut v = i.collect::<Vec<_>>();
        v.reverse();
        assert_eq!(v, i.rev().collect::<Vec<_>>());

        let i = IpAddrRange::from(Ipv4AddrRange::new(
            Ipv4Addr::from_str("255.255.255.254").unwrap(),
            Ipv4Addr::from_str("255.255.255.255").unwrap()
        ));

        assert_eq!(i.collect::<Vec<IpAddr>>(), vec![
            IpAddr::from_str("255.255.255.254").unwrap(),
            IpAddr::from_str("255.255.255.255").unwrap(),
        ]);

        let i = IpAddrRange::from(Ipv6AddrRange::new(
            Ipv6Addr::from_str("fd00::").unwrap(),
            Ipv6Addr::from_str("fd00::3").unwrap(),
        ));

        assert_eq!(i.collect::<Vec<IpAddr>>(), vec![
            IpAddr::from_str("fd00::").unwrap(),
            IpAddr::from_str("fd00::1").unwrap(),
            IpAddr::from_str("fd00::2").unwrap(),
            IpAddr::from_str("fd00::3").unwrap(),
        ]);

        let mut v = i.collect::<Vec<_>>();
        v.reverse();
        assert_eq!(v, i.rev().collect::<Vec<_>>());

        let i = IpAddrRange::from(Ipv6AddrRange::new(
            Ipv6Addr::from_str("ffff:ffff:ffff:ffff:ffff:ffff:ffff:fffe").unwrap(),
            Ipv6Addr::from_str("ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff").unwrap(),
        ));

        assert_eq!(i.collect::<Vec<IpAddr>>(), vec![
            IpAddr::from_str("ffff:ffff:ffff:ffff:ffff:ffff:ffff:fffe").unwrap(),
            IpAddr::from_str("ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff").unwrap(),
        ]);

        // #11 (infinite iterator when start and stop are 0)
        let zero4 = Ipv4Addr::from_str("0.0.0.0").unwrap();
        let zero6 = Ipv6Addr::from_str("::").unwrap();

        let mut i = Ipv4AddrRange::new(zero4, zero4);
        assert_eq!(Some(zero4), i.next());
        assert_eq!(None, i.next());

        let mut i = Ipv6AddrRange::new(zero6, zero6);
        assert_eq!(Some(zero6), i.next());
        assert_eq!(None, i.next());

        // Count
        let i = Ipv4AddrRange::new(
            Ipv4Addr::from_str("10.0.0.0").unwrap(),
            Ipv4Addr::from_str("10.0.0.3").unwrap()
        );
        assert_eq!(i.count(), 4);

        let i = Ipv6AddrRange::new(
            Ipv6Addr::from_str("fd00::").unwrap(),
            Ipv6Addr::from_str("fd00::3").unwrap(),
        );
        assert_eq!(i.count(), 4);

        // Size Hint
        let i = Ipv4AddrRange::new(
            Ipv4Addr::from_str("10.0.0.0").unwrap(),
            Ipv4Addr::from_str("10.0.0.3").unwrap()
        );
        assert_eq!(i.size_hint(), (4, Some(4)));

        let i = Ipv6AddrRange::new(
            Ipv6Addr::from_str("fd00::").unwrap(),
            Ipv6Addr::from_str("fd00::3").unwrap(),
        );
        assert_eq!(i.size_hint(), (4, Some(4)));

        // Size Hint: a range where size clearly overflows usize
        let i = Ipv6AddrRange::new(
            Ipv6Addr::from_str("::").unwrap(),
            Ipv6Addr::from_str("8000::").unwrap(),
        );
        assert_eq!(i.size_hint(), (usize::MAX, None));

        // Min, Max, Last
        let i = Ipv4AddrRange::new(
            Ipv4Addr::from_str("10.0.0.0").unwrap(),
            Ipv4Addr::from_str("10.0.0.3").unwrap()
        );
        assert_eq!(Iterator::min(i), Some(Ipv4Addr::from_str("10.0.0.0").unwrap()));
        assert_eq!(Iterator::max(i), Some(Ipv4Addr::from_str("10.0.0.3").unwrap()));
        assert_eq!(i.last(), Some(Ipv4Addr::from_str("10.0.0.3").unwrap()));

        let i = Ipv6AddrRange::new(
            Ipv6Addr::from_str("fd00::").unwrap(),
            Ipv6Addr::from_str("fd00::3").unwrap(),
        );
        assert_eq!(Iterator::min(i), Some(Ipv6Addr::from_str("fd00::").unwrap()));
        assert_eq!(Iterator::max(i), Some(Ipv6Addr::from_str("fd00::3").unwrap()));
        assert_eq!(i.last(), Some(Ipv6Addr::from_str("fd00::3").unwrap()));

        // Nth
        let i = Ipv4AddrRange::new(
            Ipv4Addr::from_str("10.0.0.0").unwrap(),
            Ipv4Addr::from_str("10.0.0.3").unwrap()
        );
        assert_eq!(i.clone().nth(0), Some(Ipv4Addr::from_str("10.0.0.0").unwrap()));
        assert_eq!(i.clone().nth(3), Some(Ipv4Addr::from_str("10.0.0.3").unwrap()));
        assert_eq!(i.clone().nth(4), None);
        assert_eq!(i.clone().nth(99), None);
        let mut i2 = i.clone();
        assert_eq!(i2.nth(1), Some(Ipv4Addr::from_str("10.0.0.1").unwrap()));
        assert_eq!(i2.nth(1), Some(Ipv4Addr::from_str("10.0.0.3").unwrap()));
        assert_eq!(i2.nth(0), None);
        let mut i3 = i.clone();
        assert_eq!(i3.nth(99), None);
        assert_eq!(i3.next(), None);

        let i = Ipv6AddrRange::new(
            Ipv6Addr::from_str("fd00::").unwrap(),
            Ipv6Addr::from_str("fd00::3").unwrap(),
        );
        assert_eq!(i.clone().nth(0), Some(Ipv6Addr::from_str("fd00::").unwrap()));
        assert_eq!(i.clone().nth(3), Some(Ipv6Addr::from_str("fd00::3").unwrap()));
        assert_eq!(i.clone().nth(4), None);
        assert_eq!(i.clone().nth(99), None);
        let mut i2 = i.clone();
        assert_eq!(i2.nth(1), Some(Ipv6Addr::from_str("fd00::1").unwrap()));
        assert_eq!(i2.nth(1), Some(Ipv6Addr::from_str("fd00::3").unwrap()));
        assert_eq!(i2.nth(0), None);
        let mut i3 = i.clone();
        assert_eq!(i3.nth(99), None);
        assert_eq!(i3.next(), None);

        // Nth Back
        let i = Ipv4AddrRange::new(
            Ipv4Addr::from_str("10.0.0.0").unwrap(),
            Ipv4Addr::from_str("10.0.0.3").unwrap()
        );
        assert_eq!(i.clone().nth_back(0), Some(Ipv4Addr::from_str("10.0.0.3").unwrap()));
        assert_eq!(i.clone().nth_back(3), Some(Ipv4Addr::from_str("10.0.0.0").unwrap()));
        assert_eq!(i.clone().nth_back(4), None);
        assert_eq!(i.clone().nth_back(99), None);
        let mut i2 = i.clone();
        assert_eq!(i2.nth_back(1), Some(Ipv4Addr::from_str("10.0.0.2").unwrap()));
        assert_eq!(i2.nth_back(1), Some(Ipv4Addr::from_str("10.0.0.0").unwrap()));
        assert_eq!(i2.nth_back(0), None);
        let mut i3 = i.clone();
        assert_eq!(i3.nth_back(99), None);
        assert_eq!(i3.next(), None);

        let i = Ipv6AddrRange::new(
            Ipv6Addr::from_str("fd00::").unwrap(),
            Ipv6Addr::from_str("fd00::3").unwrap(),
        );
        assert_eq!(i.clone().nth_back(0), Some(Ipv6Addr::from_str("fd00::3").unwrap()));
        assert_eq!(i.clone().nth_back(3), Some(Ipv6Addr::from_str("fd00::").unwrap()));
        assert_eq!(i.clone().nth_back(4), None);
        assert_eq!(i.clone().nth_back(99), None);
        let mut i2 = i.clone();
        assert_eq!(i2.nth_back(1), Some(Ipv6Addr::from_str("fd00::2").unwrap()));
        assert_eq!(i2.nth_back(1), Some(Ipv6Addr::from_str("fd00::").unwrap()));
        assert_eq!(i2.nth_back(0), None);
        let mut i3 = i.clone();
        assert_eq!(i3.nth_back(99), None);
        assert_eq!(i3.next(), None);
    }
}
