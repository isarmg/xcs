#[cfg(test)]
mod tests {
    // Typical usage
    #[test]
    #[cfg(all(feature = "webpki-roots", feature = "http1"))]
    fn test_builder() {
        ensure_global_state();
        let _connector = super::ConnectorBuilder::new()
            .with_webpki_roots()
            .https_only()
            .enable_http1()
            .build();
    }

    #[test]
    #[cfg(feature = "http1")]
    #[should_panic(expected = "ALPN protocols should not be pre-defined")]
    fn test_reject_predefined_alpn() {
        ensure_global_state();
        let roots = rustls::RootCertStore::empty();
        let mut config_with_alpn = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        config_with_alpn.alpn_protocols = vec![b"fancyprotocol".to_vec()];
        let _connector = super::ConnectorBuilder::new()
            .with_tls_config(config_with_alpn)
            .https_only()
            .enable_http1()
            .build();
    }

    #[test]
    #[cfg(all(feature = "http1", feature = "http2"))]
    fn test_alpn() {
        ensure_global_state();
        let roots = rustls::RootCertStore::empty();
        let tls_config = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        let connector = super::ConnectorBuilder::new()
            .with_tls_config(tls_config.clone())
            .https_only()
            .enable_http1()
            .build();
        assert!(connector
            .tls_config
            .alpn_protocols
            .is_empty());
        let connector = super::ConnectorBuilder::new()
            .with_tls_config(tls_config.clone())
            .https_only()
            .enable_http2()
            .build();
        assert_eq!(&connector.tls_config.alpn_protocols, &[b"h2".to_vec()]);
        let connector = super::ConnectorBuilder::new()
            .with_tls_config(tls_config.clone())
            .https_only()
            .enable_http1()
            .enable_http2()
            .build();
        assert_eq!(
            &connector.tls_config.alpn_protocols,
            &[b"h2".to_vec(), b"http/1.1".to_vec()]
        );
        let connector = super::ConnectorBuilder::new()
            .with_tls_config(tls_config)
            .https_only()
            .enable_all_versions()
            .build();
        assert_eq!(
            &connector.tls_config.alpn_protocols,
            &[b"h2".to_vec(), b"http/1.1".to_vec()]
        );
    }

    #[test]
    #[cfg(all(not(feature = "http1"), feature = "http2"))]
    fn test_alpn_http2() {
        let roots = rustls::RootCertStore::empty();
        let tls_config = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(roots)
            .with_no_client_auth();
        let connector = super::ConnectorBuilder::new()
            .with_tls_config(tls_config.clone())
            .https_only()
            .enable_http2()
            .build();
        assert_eq!(&connector.tls_config.alpn_protocols, &[b"h2".to_vec()]);
        let connector = super::ConnectorBuilder::new()
            .with_tls_config(tls_config)
            .https_only()
            .enable_all_versions()
            .build();
        assert_eq!(&connector.tls_config.alpn_protocols, &[b"h2".to_vec()]);
    }

    fn ensure_global_state() {
        #[cfg(feature = "ring")]
        let _ = rustls::crypto::ring::default_provider().install_default();
        #[cfg(feature = "aws-lc-rs")]
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    }
}
