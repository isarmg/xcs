#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ee_read_for_rpk() {
        // Try to read an end entity certificate into a RawPublicKeyEntity.
        // It will fail to parse the key value since we expect no unused bits.
        let ee = include_bytes!("../tests/ed25519/ee.der");
        let ee_der = SubjectPublicKeyInfoDer::from(ee.as_slice());
        assert_eq!(
            RawPublicKeyEntity::try_from(&ee_der).expect_err("unexpectedly parsed certificate"),
            Error::TrailingData(DerTypeId::BitString)
        );
    }

    #[test]
    fn test_spki_read_for_rpk() {
        let pubkey = include_bytes!("../tests/ed25519/ee-pubkey.der");
        let spki_der = SubjectPublicKeyInfoDer::from(pubkey.as_slice());
        let rpk = RawPublicKeyEntity::try_from(&spki_der).expect("failed to parse rpk");

        // Retrieved the SPKI from the pubkey.der using the following commands (as in [`cert::test_spki_read`]):
        // xxd -plain -cols 1 tests/ed255519/ee-pubkey.der
        let expected_spki = [
            0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00, 0xfe, 0x5a, 0x1e, 0x36,
            0x6c, 0x17, 0x27, 0x5b, 0xf1, 0x58, 0x1e, 0x3a, 0x0e, 0xe6, 0x56, 0x29, 0x8d, 0x9e,
            0x1b, 0x3f, 0xd3, 0x3f, 0x96, 0x46, 0xef, 0xbf, 0x04, 0x6b, 0xc7, 0x3d, 0x47, 0x5c,
        ];
        assert_eq!(expected_spki, rpk.inner.as_slice_less_safe())
    }
}
