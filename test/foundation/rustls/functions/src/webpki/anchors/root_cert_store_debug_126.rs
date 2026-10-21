#[test]
fn root_cert_store_debug() {
    use core::iter;

    use pki_types::Der;

    let ta = TrustAnchor {
        subject: Der::from_slice(&[]),
        subject_public_key_info: Der::from_slice(&[]),
        name_constraints: None,
    };
    let store = RootCertStore::from_iter(iter::repeat(ta).take(138));

    assert_eq!(
        format!("{store:?}"),
        "RootCertStore { roots: \"(138 roots)\" }"
    );
}
