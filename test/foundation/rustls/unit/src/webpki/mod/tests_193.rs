#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    #[test]
    fn pki_crl_errors() {
        // CRL signature errors should be turned into BadSignature.
        assert_eq!(
            pki_error(webpki::Error::InvalidCrlSignatureForPublicKey),
            Error::InvalidCertRevocationList(CertRevocationListError::BadSignature),
        );

        #[allow(deprecated)]
        {
            assert_eq!(
                pki_error(webpki::Error::UnsupportedCrlSignatureAlgorithm),
                Error::InvalidCertRevocationList(
                    CertRevocationListError::UnsupportedSignatureAlgorithm
                ),
            );
            assert_eq!(
                pki_error(webpki::Error::UnsupportedCrlSignatureAlgorithmForPublicKey),
                Error::InvalidCertRevocationList(
                    CertRevocationListError::UnsupportedSignatureAlgorithm
                ),
            );
        }

        assert_eq!(
            pki_error(webpki::Error::UnsupportedCrlSignatureAlgorithmContext(
                webpki::UnsupportedSignatureAlgorithmContext {
                    signature_algorithm_id: vec![],
                    supported_algorithms: vec![],
                }
            )),
            Error::InvalidCertRevocationList(
                CertRevocationListError::UnsupportedSignatureAlgorithmContext {
                    signature_algorithm_id: vec![],
                    supported_algorithms: vec![],
                }
            )
        );
        assert_eq!(
            pki_error(
                webpki::Error::UnsupportedCrlSignatureAlgorithmForPublicKeyContext(
                    webpki::UnsupportedSignatureAlgorithmForPublicKeyContext {
                        signature_algorithm_id: vec![],
                        public_key_algorithm_id: vec![],
                    }
                )
            ),
            Error::InvalidCertRevocationList(
                CertRevocationListError::UnsupportedSignatureAlgorithmForPublicKeyContext {
                    signature_algorithm_id: vec![],
                    public_key_algorithm_id: vec![],
                }
            )
        );

        // Revoked cert errors should be turned into Revoked.
        assert_eq!(
            pki_error(webpki::Error::CertRevoked),
            Error::InvalidCertificate(CertificateError::Revoked),
        );

        // Issuer not CRL signer errors should be turned into IssuerInvalidForCrl
        assert_eq!(
            pki_error(webpki::Error::IssuerNotCrlSigner),
            Error::InvalidCertRevocationList(CertRevocationListError::IssuerInvalidForCrl)
        );
    }

    #[test]
    fn crl_error_from_webpki() {
        use CertRevocationListError::*;
        #[allow(deprecated)]
        let testcases = &[
            (webpki::Error::InvalidCrlSignatureForPublicKey, BadSignature),
            (
                webpki::Error::UnsupportedCrlSignatureAlgorithm,
                UnsupportedSignatureAlgorithm,
            ),
            (
                webpki::Error::UnsupportedCrlSignatureAlgorithmForPublicKey,
                UnsupportedSignatureAlgorithm,
            ),
            (webpki::Error::InvalidCrlNumber, InvalidCrlNumber),
            (
                webpki::Error::InvalidSerialNumber,
                InvalidRevokedCertSerialNumber,
            ),
            (webpki::Error::IssuerNotCrlSigner, IssuerInvalidForCrl),
            (webpki::Error::MalformedExtensions, ParseError),
            (webpki::Error::BadDer, ParseError),
            (webpki::Error::BadDerTime, ParseError),
            (
                webpki::Error::UnsupportedCriticalExtension,
                UnsupportedCriticalExtension,
            ),
            (webpki::Error::UnsupportedCrlVersion, UnsupportedCrlVersion),
            (webpki::Error::UnsupportedDeltaCrl, UnsupportedDeltaCrl),
            (
                webpki::Error::UnsupportedIndirectCrl,
                UnsupportedIndirectCrl,
            ),
            (
                webpki::Error::UnsupportedRevocationReason,
                UnsupportedRevocationReason,
            ),
        ];
        for t in testcases {
            assert_eq!(crl_error(t.0.clone()), t.1);
        }

        assert!(matches!(
            crl_error(webpki::Error::NameConstraintViolation),
            Other(..)
        ));
    }
}
