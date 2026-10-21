#[test]
fn assertions_are_debug() {
    use std::format;

    assert_eq!(
        format!("{:?}", ClientCertVerified::assertion()),
        "ClientCertVerified(())"
    );
    assert_eq!(
        format!("{:?}", HandshakeSignatureValid::assertion()),
        "HandshakeSignatureValid(())"
    );
    assert_eq!(
        format!("{:?}", FinishedMessageVerified::assertion()),
        "FinishedMessageVerified(())"
    );
    assert_eq!(
        format!("{:?}", ServerCertVerified::assertion()),
        "ServerCertVerified(())"
    );
}
