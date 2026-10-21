#[cfg(all(
    feature = "aws-lc-rs",
    feature = "prefer-post-quantum",
    not(feature = "fips")
))]
#[test]
fn hybrid_kx_component_share_offered_if_supported_separately() {
    let ch = client_hello_sent_for_config(
        ClientConfig::builder_with_provider(crate::crypto::aws_lc_rs::default_provider().into())
            .with_safe_default_protocol_versions()
            .unwrap()
            .with_root_certificates(roots())
            .with_no_client_auth(),
    )
    .unwrap();

    let key_shares = ch
        .extensions
        .key_shares
        .as_ref()
        .unwrap();
    assert_eq!(key_shares.len(), 2);
    assert_eq!(key_shares[0].group, NamedGroup::X25519MLKEM768);
    assert_eq!(key_shares[1].group, NamedGroup::X25519);
}
