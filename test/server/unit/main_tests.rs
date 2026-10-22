use super::*;
#[test]
fn setup_maps_validate_syntax_without_accessing_missing_files() {
    assert!(
        parse_setup_map(
            vec!["preset=/definitely/missing/config.json".into()],
            "--setup-config"
        )
        .is_ok()
    );
    for values in [
        vec!["preset=relative".into()],
        vec!["=/absolute".into()],
        vec!["same=/a".into(), "same=/b".into()],
    ] {
        assert!(parse_setup_map(values, "--setup-config").is_err());
    }
}
#[test]
fn trusted_proxy_requires_origin_and_exact_ip() {
    assert!(Args::try_parse_from(["zxs", "--trusted-proxy-ip", "127.0.0.1"]).is_err());
    assert!(
        Args::try_parse_from([
            "zxs",
            "--external-origin",
            "https://storage.example.org",
            "--trusted-proxy-ip",
            "127.0.0.0/8"
        ])
        .is_err()
    );
    let args = Args::try_parse_from([
        "zxs",
        "--external-origin",
        "https://storage.example.org",
        "--trusted-proxy-ip",
        "127.0.0.1",
        "--trusted-proxy-ip",
        "::1",
    ])
    .unwrap();
    assert_eq!(args.trusted_proxy_ips.len(), 2);
}
