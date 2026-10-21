#[cfg(test)]
mod tests {
    use std::io;
    use std::net::SocketAddr;

    use ::http::Uri;

    use crate::adapters::client::legacy::connect::http::TcpKeepaliveConfig;

    use super::super::sealed::{Connect, ConnectSvc};
    use super::{Config, ConnectError, HttpConnector};

    use super::set_port;

    async fn connect<C>(
        connector: C,
        dst: Uri,
    ) -> Result<<C::_Svc as ConnectSvc>::Connection, <C::_Svc as ConnectSvc>::Error>
    where
        C: Connect,
    {
        connector.connect(super::super::sealed::Internal, dst).await
    }

    #[tokio::test]
    async fn test_errors_enforce_http() {
        let dst = "https://example.domain/foo/bar?baz".parse().unwrap();
        let connector = HttpConnector::new();

        let err = connect(connector, dst).await.unwrap_err();
        assert_eq!(&*err.msg, super::INVALID_NOT_HTTP);
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn get_local_ips() -> (Option<std::net::Ipv4Addr>, Option<std::net::Ipv6Addr>) {
        use std::net::{IpAddr, TcpListener};

        let mut ip_v4 = None;
        let mut ip_v6 = None;

        let ips = pnet_datalink::interfaces()
            .into_iter()
            .flat_map(|i| i.ips.into_iter().map(|n| n.ip()));

        for ip in ips {
            match ip {
                IpAddr::V4(ip) if TcpListener::bind((ip, 0)).is_ok() => ip_v4 = Some(ip),
                IpAddr::V6(ip) if TcpListener::bind((ip, 0)).is_ok() => ip_v6 = Some(ip),
                _ => (),
            }

            if ip_v4.is_some() && ip_v6.is_some() {
                break;
            }
        }

        (ip_v4, ip_v6)
    }

    #[cfg(any(target_os = "android", target_os = "fuchsia", target_os = "linux"))]
    fn default_interface() -> Option<String> {
        pnet_datalink::interfaces()
            .iter()
            .find(|e| e.is_up() && !e.is_loopback() && !e.ips.is_empty())
            .map(|e| e.name.clone())
    }

    #[tokio::test]
    async fn test_errors_missing_scheme() {
        let dst = "example.domain".parse().unwrap();
        let mut connector = HttpConnector::new();
        connector.enforce_http(false);

        let err = connect(connector, dst).await.unwrap_err();
        assert_eq!(&*err.msg, super::INVALID_MISSING_SCHEME);
    }

    // NOTE: pnet crate that we use in this test doesn't compile on Windows
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[cfg_attr(miri, ignore)]
    #[tokio::test]
    async fn local_address() {
        use std::net::{IpAddr, TcpListener};

        let (bind_ip_v4, bind_ip_v6) = get_local_ips();
        let server4 = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = server4.local_addr().unwrap().port();
        let server6 = TcpListener::bind(format!("[::1]:{port}")).unwrap();

        let assert_client_ip = |dst: String, server: TcpListener, expected_ip: IpAddr| async move {
            let mut connector = HttpConnector::new();

            match (bind_ip_v4, bind_ip_v6) {
                (Some(v4), Some(v6)) => connector.set_local_addresses(v4, v6),
                (Some(v4), None) => connector.set_local_address(Some(v4.into())),
                (None, Some(v6)) => connector.set_local_address(Some(v6.into())),
                _ => unreachable!(),
            }

            connect(connector, dst.parse().unwrap()).await.unwrap();

            let (_, client_addr) = server.accept().unwrap();

            assert_eq!(client_addr.ip(), expected_ip);
        };

        if let Some(ip) = bind_ip_v4 {
            assert_client_ip(format!("http://127.0.0.1:{port}"), server4, ip.into()).await;
        }

        if let Some(ip) = bind_ip_v6 {
            assert_client_ip(format!("http://[::1]:{port}"), server6, ip.into()).await;
        }
    }

    // NOTE: pnet crate that we use in this test doesn't compile on Windows
    #[cfg(any(target_os = "android", target_os = "fuchsia", target_os = "linux"))]
    #[tokio::test]
    #[ignore = "setting `SO_BINDTODEVICE` requires the `CAP_NET_RAW` capability (works when running as root)"]
    async fn interface() {
        use socket2::{Domain, Protocol, Socket, Type};
        use std::net::TcpListener;

        let interface: Option<String> = default_interface();

        let server4 = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = server4.local_addr().unwrap().port();

        let server6 = TcpListener::bind(format!("[::1]:{port}")).unwrap();

        let assert_interface_name =
            |dst: String,
             server: TcpListener,
             bind_iface: Option<String>,
             expected_interface: Option<String>| async move {
                let mut connector = HttpConnector::new();
                if let Some(iface) = bind_iface {
                    connector.set_interface(iface);
                }

                connect(connector, dst.parse().unwrap()).await.unwrap();
                let domain = Domain::for_address(server.local_addr().unwrap());
                let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP)).unwrap();

                assert_eq!(
                    socket.device().unwrap().as_deref(),
                    expected_interface.as_deref().map(|val| val.as_bytes())
                );
            };

        assert_interface_name(
            format!("http://127.0.0.1:{port}"),
            server4,
            interface.clone(),
            interface.clone(),
        )
        .await;
        assert_interface_name(
            format!("http://[::1]:{port}"),
            server6,
            interface.clone(),
            interface.clone(),
        )
        .await;
    }

    #[test]
    #[ignore] // TODO
    #[cfg_attr(not(feature = "__internal_happy_eyeballs_tests"), ignore)]
    fn client_happy_eyeballs() {
        use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, TcpListener};
        use std::time::{Duration, Instant};

        use super::ConnectingTcp;
        use super::dns;

        let server4 = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = server4.local_addr().unwrap();
        let _server6 = TcpListener::bind(format!("[::1]:{}", addr.port())).unwrap();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let local_timeout = Duration::default();
        let unreachable_v4_timeout = measure_connect(unreachable_ipv4_addr()).1;
        let unreachable_v6_timeout = measure_connect(unreachable_ipv6_addr()).1;
        let fallback_timeout = std::cmp::max(unreachable_v4_timeout, unreachable_v6_timeout)
            + Duration::from_millis(250);

        let scenarios = &[
            // Fast primary, without fallback.
            (&[local_ipv4_addr()][..], 4, local_timeout, false),
            (&[local_ipv6_addr()][..], 6, local_timeout, false),
            // Fast primary, with (unused) fallback.
            (
                &[local_ipv4_addr(), local_ipv6_addr()][..],
                4,
                local_timeout,
                false,
            ),
            (
                &[local_ipv6_addr(), local_ipv4_addr()][..],
                6,
                local_timeout,
                false,
            ),
            // Unreachable + fast primary, without fallback.
            (
                &[unreachable_ipv4_addr(), local_ipv4_addr()][..],
                4,
                unreachable_v4_timeout,
                false,
            ),
            (
                &[unreachable_ipv6_addr(), local_ipv6_addr()][..],
                6,
                unreachable_v6_timeout,
                false,
            ),
            // Unreachable + fast primary, with (unused) fallback.
            (
                &[
                    unreachable_ipv4_addr(),
                    local_ipv4_addr(),
                    local_ipv6_addr(),
                ][..],
                4,
                unreachable_v4_timeout,
                false,
            ),
            (
                &[
                    unreachable_ipv6_addr(),
                    local_ipv6_addr(),
                    local_ipv4_addr(),
                ][..],
                6,
                unreachable_v6_timeout,
                true,
            ),
            // Slow primary, with (used) fallback.
            (
                &[slow_ipv4_addr(), local_ipv4_addr(), local_ipv6_addr()][..],
                6,
                fallback_timeout,
                false,
            ),
            (
                &[slow_ipv6_addr(), local_ipv6_addr(), local_ipv4_addr()][..],
                4,
                fallback_timeout,
                true,
            ),
            // Slow primary, with (used) unreachable + fast fallback.
            (
                &[slow_ipv4_addr(), unreachable_ipv6_addr(), local_ipv6_addr()][..],
                6,
                fallback_timeout + unreachable_v6_timeout,
                false,
            ),
            (
                &[slow_ipv6_addr(), unreachable_ipv4_addr(), local_ipv4_addr()][..],
                4,
                fallback_timeout + unreachable_v4_timeout,
                true,
            ),
        ];

        // Scenarios for IPv6 -> IPv4 fallback require that host can access IPv6 network.
        // Otherwise, connection to "slow" IPv6 address will error-out immediately.
        let ipv6_accessible = measure_connect(slow_ipv6_addr()).0;

        for &(hosts, family, timeout, needs_ipv6_access) in scenarios {
            if needs_ipv6_access && !ipv6_accessible {
                continue;
            }

            let (start, stream) = rt
                .block_on(async move {
                    let addrs = hosts
                        .iter()
                        .map(|host| (*host, addr.port()).into())
                        .collect();
                    let cfg = Config {
                        local_address_ipv4: None,
                        local_address_ipv6: None,
                        connect_timeout: None,
                        tcp_keepalive_config: TcpKeepaliveConfig::default(),
                        happy_eyeballs_timeout: Some(fallback_timeout),
                        nodelay: false,
                        reuse_address: false,
                        enforce_http: false,
                        send_buffer_size: None,
                        recv_buffer_size: None,
                        #[cfg(any(
                            target_os = "android",
                            target_os = "fuchsia",
                            target_os = "linux"
                        ))]
                        interface: None,
                        #[cfg(any(
                            target_os = "illumos",
                            target_os = "ios",
                            target_os = "macos",
                            target_os = "solaris",
                            target_os = "tvos",
                            target_os = "visionos",
                            target_os = "watchos",
                        ))]
                        interface: None,
                        #[cfg(any(
                            target_os = "android",
                            target_os = "fuchsia",
                            target_os = "linux"
                        ))]
                        mark: None,
                        #[cfg(any(
                            target_os = "android",
                            target_os = "fuchsia",
                            target_os = "linux"
                        ))]
                        tcp_user_timeout: None,
                    };
                    let connecting_tcp = ConnectingTcp::new(dns::SocketAddrs::new(addrs), &cfg);
                    let start = Instant::now();
                    Ok::<_, ConnectError>((start, ConnectingTcp::connect(connecting_tcp).await?))
                })
                .unwrap();
            let res = if stream.peer_addr().unwrap().is_ipv4() {
                4
            } else {
                6
            };
            let duration = start.elapsed();

            // Allow actual duration to be +/- 150ms off.
            let min_duration = if timeout >= Duration::from_millis(150) {
                timeout - Duration::from_millis(150)
            } else {
                Duration::default()
            };
            let max_duration = timeout + Duration::from_millis(150);

            assert_eq!(res, family);
            assert!(duration >= min_duration);
            assert!(duration <= max_duration);
        }

        fn local_ipv4_addr() -> IpAddr {
            Ipv4Addr::new(127, 0, 0, 1).into()
        }

        fn local_ipv6_addr() -> IpAddr {
            Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1).into()
        }

        fn unreachable_ipv4_addr() -> IpAddr {
            Ipv4Addr::new(127, 0, 0, 2).into()
        }

        fn unreachable_ipv6_addr() -> IpAddr {
            Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 2).into()
        }

        fn slow_ipv4_addr() -> IpAddr {
            // RFC 6890 reserved IPv4 address.
            Ipv4Addr::new(198, 18, 0, 25).into()
        }

        fn slow_ipv6_addr() -> IpAddr {
            // RFC 6890 reserved IPv6 address.
            Ipv6Addr::new(2001, 2, 0, 0, 0, 0, 0, 254).into()
        }

        fn measure_connect(addr: IpAddr) -> (bool, Duration) {
            let start = Instant::now();
            let result =
                std::net::TcpStream::connect_timeout(&(addr, 80).into(), Duration::from_secs(1));

            let reachable = result.is_ok() || result.unwrap_err().kind() == io::ErrorKind::TimedOut;
            let duration = start.elapsed();
            (reachable, duration)
        }
    }

    use std::time::Duration;

    #[test]
    fn no_tcp_keepalive_config() {
        assert!(TcpKeepaliveConfig::default().into_tcpkeepalive().is_none());
    }

    #[test]
    fn tcp_keepalive_time_config() {
        let kac = TcpKeepaliveConfig {
            time: Some(Duration::from_secs(60)),
            ..Default::default()
        };
        if let Some(tcp_keepalive) = kac.into_tcpkeepalive() {
            assert!(format!("{tcp_keepalive:?}").contains("time: Some(60s)"));
        } else {
            panic!("test failed");
        }
    }

    #[cfg(not(any(target_os = "openbsd", target_os = "redox", target_os = "solaris")))]
    #[test]
    fn tcp_keepalive_interval_config() {
        let kac = TcpKeepaliveConfig {
            interval: Some(Duration::from_secs(1)),
            ..Default::default()
        };
        if let Some(tcp_keepalive) = kac.into_tcpkeepalive() {
            assert!(format!("{tcp_keepalive:?}").contains("interval: Some(1s)"));
        } else {
            panic!("test failed");
        }
    }

    #[cfg(not(any(
        target_os = "openbsd",
        target_os = "redox",
        target_os = "solaris",
        target_os = "windows"
    )))]
    #[test]
    fn tcp_keepalive_retries_config() {
        let kac = TcpKeepaliveConfig {
            retries: Some(3),
            ..Default::default()
        };
        if let Some(tcp_keepalive) = kac.into_tcpkeepalive() {
            assert!(format!("{tcp_keepalive:?}").contains("retries: Some(3)"));
        } else {
            panic!("test failed");
        }
    }

    #[test]
    fn test_set_port() {
        // Respect explicit ports no matter what the resolved port is.
        let mut addr = SocketAddr::from(([0, 0, 0, 0], 6881));
        set_port(&mut addr, 42, true);
        assert_eq!(addr.port(), 42);

        // Ignore default  host port, and use the socket port instead.
        let mut addr = SocketAddr::from(([0, 0, 0, 0], 6881));
        set_port(&mut addr, 443, false);
        assert_eq!(addr.port(), 6881);

        // Use the default port if the resolved port is `0`.
        let mut addr = SocketAddr::from(([0, 0, 0, 0], 0));
        set_port(&mut addr, 443, false);
        assert_eq!(addr.port(), 443);
    }
}
