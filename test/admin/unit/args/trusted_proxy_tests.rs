use super::*;

#[test]
fn trusted_proxy_is_explicit_repeatable_and_requires_origin() {
    let args = Args::try_parse_from(["zxs", "serve"]).unwrap();
    let Command::Serve {
        trusted_proxy_ips, ..
    } = args.command
    else {
        panic!("serve expected")
    };
    assert!(trusted_proxy_ips.is_empty());
    assert!(Args::try_parse_from(["zxs", "serve", "--trusted-proxy-ip", "127.0.0.1"]).is_err());
    assert!(
        Args::try_parse_from([
            "zxs",
            "serve",
            "--external-origin",
            "https://storage.example.org",
            "--trusted-proxy-ip",
            "127.0.0.0/8"
        ])
        .is_err()
    );
    let args = Args::try_parse_from([
        "zxs",
        "serve",
        "--external-origin",
        "https://storage.example.org",
        "--trusted-proxy-ip",
        "127.0.0.1",
        "--trusted-proxy-ip",
        "::1",
    ])
    .unwrap();
    let Command::Serve {
        trusted_proxy_ips, ..
    } = args.command
    else {
        panic!("serve expected")
    };
    assert_eq!(
        trusted_proxy_ips,
        vec![
            "127.0.0.1".parse::<std::net::IpAddr>().unwrap(),
            "::1".parse().unwrap()
        ]
    );
}
