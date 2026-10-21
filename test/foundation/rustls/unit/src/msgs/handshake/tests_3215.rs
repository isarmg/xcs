#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ech_config_dupe_exts() {
        let unknown_ext = EchConfigExtension::Unknown(UnknownExtension {
            typ: ExtensionType::Unknown(0x42),
            payload: Payload::new(vec![0x42]),
        });
        let mut config = config_template();
        config
            .extensions
            .push(unknown_ext.clone());
        config.extensions.push(unknown_ext);

        assert!(config.has_duplicate_extension());
        assert!(!config.has_unknown_mandatory_extension());
    }

    #[test]
    fn test_ech_config_mandatory_exts() {
        let mandatory_unknown_ext = EchConfigExtension::Unknown(UnknownExtension {
            typ: ExtensionType::Unknown(0x42 | 0x8000), // Note: high bit set.
            payload: Payload::new(vec![0x42]),
        });
        let mut config = config_template();
        config
            .extensions
            .push(mandatory_unknown_ext);

        assert!(!config.has_duplicate_extension());
        assert!(config.has_unknown_mandatory_extension());
    }

    fn config_template() -> EchConfigContents {
        EchConfigContents {
            key_config: HpkeKeyConfig {
                config_id: 0,
                kem_id: HpkeKem::DHKEM_P256_HKDF_SHA256,
                public_key: PayloadU16::new(b"xxx".into()),
                symmetric_cipher_suites: vec![HpkeSymmetricCipherSuite {
                    kdf_id: HpkeKdf::HKDF_SHA256,
                    aead_id: HpkeAead::AES_128_GCM,
                }],
            },
            maximum_name_length: 0,
            public_name: DnsName::try_from("example.com").unwrap(),
            extensions: vec![],
        }
    }
}
