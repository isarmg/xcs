#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipv4() {
        use std::net::Ipv4Addr;
        let std = SocketAddrV4::new(Ipv4Addr::new(1, 2, 3, 4), 9876);
        let addr = SockAddr::from(std);
        assert!(addr.is_ipv4());
        assert!(!addr.is_ipv6());
        #[cfg(not(target_os = "wasi"))]
        assert!(!addr.is_unix());
        assert_eq!(addr.family(), AF_INET as sa_family_t);
        assert_eq!(addr.domain(), Domain::IPV4);
        assert_eq!(addr.len(), size_of::<sockaddr_in>() as socklen_t);
        assert_eq!(addr.as_socket(), Some(SocketAddr::V4(std)));
        assert_eq!(addr.as_socket_ipv4(), Some(std));
        assert!(addr.as_socket_ipv6().is_none());

        let addr = SockAddr::from(SocketAddr::from(std));
        assert_eq!(addr.family(), AF_INET as sa_family_t);
        assert_eq!(addr.len(), size_of::<sockaddr_in>() as socklen_t);
        assert_eq!(addr.as_socket(), Some(SocketAddr::V4(std)));
        assert_eq!(addr.as_socket_ipv4(), Some(std));
        assert!(addr.as_socket_ipv6().is_none());
        #[cfg(all(unix, not(target_os = "wasi")))]
        {
            assert!(addr.as_pathname().is_none());
            assert!(addr.as_abstract_namespace().is_none());
        }
    }

    #[test]
    fn ipv6() {
        use std::net::Ipv6Addr;
        let std = SocketAddrV6::new(Ipv6Addr::new(1, 2, 3, 4, 5, 6, 7, 8), 9876, 11, 12);
        let addr = SockAddr::from(std);
        assert!(addr.is_ipv6());
        assert!(!addr.is_ipv4());
        #[cfg(not(target_os = "wasi"))]
        assert!(!addr.is_unix());
        assert_eq!(addr.family(), AF_INET6 as sa_family_t);
        assert_eq!(addr.domain(), Domain::IPV6);
        assert_eq!(addr.len(), size_of::<sockaddr_in6>() as socklen_t);
        assert_eq!(addr.as_socket(), Some(SocketAddr::V6(std)));
        assert!(addr.as_socket_ipv4().is_none());
        assert_eq!(addr.as_socket_ipv6(), Some(std));

        let addr = SockAddr::from(SocketAddr::from(std));
        assert_eq!(addr.family(), AF_INET6 as sa_family_t);
        assert_eq!(addr.len(), size_of::<sockaddr_in6>() as socklen_t);
        assert_eq!(addr.as_socket(), Some(SocketAddr::V6(std)));
        assert!(addr.as_socket_ipv4().is_none());
        assert_eq!(addr.as_socket_ipv6(), Some(std));
        #[cfg(all(unix, not(target_os = "wasi")))]
        {
            assert!(addr.as_pathname().is_none());
            assert!(addr.as_abstract_namespace().is_none());
        }
    }

    #[test]
    fn ipv4_eq() {
        use std::net::Ipv4Addr;

        let std1 = SocketAddrV4::new(Ipv4Addr::new(1, 2, 3, 4), 9876);
        let std2 = SocketAddrV4::new(Ipv4Addr::new(5, 6, 7, 8), 8765);

        test_eq(
            SockAddr::from(std1),
            SockAddr::from(std1),
            SockAddr::from(std2),
        );
    }

    #[test]
    fn ipv4_hash() {
        use std::net::Ipv4Addr;

        let std1 = SocketAddrV4::new(Ipv4Addr::new(1, 2, 3, 4), 9876);
        let std2 = SocketAddrV4::new(Ipv4Addr::new(5, 6, 7, 8), 8765);

        test_hash(
            SockAddr::from(std1),
            SockAddr::from(std1),
            SockAddr::from(std2),
        );
    }

    #[test]
    fn ipv6_eq() {
        use std::net::Ipv6Addr;

        let std1 = SocketAddrV6::new(Ipv6Addr::new(1, 2, 3, 4, 5, 6, 7, 8), 9876, 11, 12);
        let std2 = SocketAddrV6::new(Ipv6Addr::new(3, 4, 5, 6, 7, 8, 9, 0), 7654, 13, 14);

        test_eq(
            SockAddr::from(std1),
            SockAddr::from(std1),
            SockAddr::from(std2),
        );
    }

    #[test]
    fn ipv6_hash() {
        use std::net::Ipv6Addr;

        let std1 = SocketAddrV6::new(Ipv6Addr::new(1, 2, 3, 4, 5, 6, 7, 8), 9876, 11, 12);
        let std2 = SocketAddrV6::new(Ipv6Addr::new(3, 4, 5, 6, 7, 8, 9, 0), 7654, 13, 14);

        test_hash(
            SockAddr::from(std1),
            SockAddr::from(std1),
            SockAddr::from(std2),
        );
    }

    #[test]
    fn ipv4_ipv6_eq() {
        use std::net::Ipv4Addr;
        use std::net::Ipv6Addr;

        let std1 = SocketAddrV4::new(Ipv4Addr::new(1, 2, 3, 4), 9876);
        let std2 = SocketAddrV6::new(Ipv6Addr::new(1, 2, 3, 4, 5, 6, 7, 8), 9876, 11, 12);

        test_eq(
            SockAddr::from(std1),
            SockAddr::from(std1),
            SockAddr::from(std2),
        );

        test_eq(
            SockAddr::from(std2),
            SockAddr::from(std2),
            SockAddr::from(std1),
        );
    }

    #[test]
    fn ipv4_ipv6_hash() {
        use std::net::Ipv4Addr;
        use std::net::Ipv6Addr;

        let std1 = SocketAddrV4::new(Ipv4Addr::new(1, 2, 3, 4), 9876);
        let std2 = SocketAddrV6::new(Ipv6Addr::new(1, 2, 3, 4, 5, 6, 7, 8), 9876, 11, 12);

        test_hash(
            SockAddr::from(std1),
            SockAddr::from(std1),
            SockAddr::from(std2),
        );

        test_hash(
            SockAddr::from(std2),
            SockAddr::from(std2),
            SockAddr::from(std1),
        );
    }

    #[allow(clippy::eq_op)] // allow a0 == a0 check
    fn test_eq(a0: SockAddr, a1: SockAddr, b: SockAddr) {
        assert!(a0 == a0);
        assert!(a0 == a1);
        assert!(a1 == a0);
        assert!(a0 != b);
        assert!(b != a0);
    }

    fn test_hash(a0: SockAddr, a1: SockAddr, b: SockAddr) {
        assert!(calculate_hash(&a0) == calculate_hash(&a0));
        assert!(calculate_hash(&a0) == calculate_hash(&a1));
        // technically unequal values can have the same hash, in this case x != z and both have different hashes
        assert!(calculate_hash(&a0) != calculate_hash(&b));
    }

    fn calculate_hash(x: &SockAddr) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher;

        let mut hasher = DefaultHasher::new();
        x.hash(&mut hasher);
        hasher.finish()
    }
}
