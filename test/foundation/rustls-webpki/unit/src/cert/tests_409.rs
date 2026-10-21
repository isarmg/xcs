#[cfg(test)]
mod tests {
    #[cfg(feature = "alloc")]
    use alloc::vec::Vec;

    use super::*;
    #[cfg(feature = "alloc")]
    use crate::crl::RevocationReason;

    #[test]
    // Note: cert::parse_cert is crate-local visibility, and EndEntityCert doesn't expose the
    //       inner Cert, or the serial number. As a result we test that the raw serial value
    //       is read correctly here instead of in tests/integration.rs.
    fn test_serial_read() {
        let ee = include_bytes!("../tests/misc/serial_neg_ee.der");
        let cert = Cert::from_der(untrusted::Input::from(ee)).expect("failed to parse certificate");
        assert_eq!(cert.serial.as_slice_less_safe(), &[255, 33, 82, 65, 17]);

        let ee = include_bytes!("../tests/misc/serial_large_positive.der");
        let cert = Cert::from_der(untrusted::Input::from(ee)).expect("failed to parse certificate");
        assert_eq!(
            cert.serial.as_slice_less_safe(),
            &[
                0, 230, 9, 254, 122, 234, 0, 104, 140, 224, 36, 180, 237, 32, 27, 31, 239, 82, 180,
                68, 209
            ]
        )
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_spki_read() {
        let ee = include_bytes!("../tests/ed25519/ee.der");
        let cert = Cert::from_der(untrusted::Input::from(ee)).expect("failed to parse certificate");
        // How did I get this lovely string of hex bytes?
        // openssl x509 -in tests/ed25519/ee.der -pubkey -noout > pubkey.pem
        // openssl ec -pubin -in pubkey.pem -outform DER -out pubkey.der
        // xxd -plain -cols 1 pubkey.der
        let expected_spki = [
            0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00, 0xfe, 0x5a,
            0x1e, 0x36, 0x6c, 0x17, 0x27, 0x5b, 0xf1, 0x58, 0x1e, 0x3a, 0x0e, 0xe6, 0x56, 0x29,
            0x8d, 0x9e, 0x1b, 0x3f, 0xd3, 0x3f, 0x96, 0x46, 0xef, 0xbf, 0x04, 0x6b, 0xc7, 0x3d,
            0x47, 0x5c,
        ];
        assert_eq!(expected_spki, *cert.subject_public_key_info())
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn test_crl_distribution_point_netflix() {
        let ee = include_bytes!("../tests/netflix/ee.der");
        let inter = include_bytes!("../tests/netflix/inter.der");
        let ee_cert = Cert::from_der(untrusted::Input::from(ee)).expect("failed to parse EE cert");
        let cert =
            Cert::from_der(untrusted::Input::from(inter)).expect("failed to parse certificate");

        // The end entity certificate shouldn't have a distribution point.
        assert!(ee_cert.crl_distribution_points.is_none());

        // We expect to be able to parse the intermediate certificate's CRL distribution points.
        let crl_distribution_points = cert
            .crl_distribution_points()
            .expect("missing distribution points extension")
            .collect::<Result<Vec<_>, Error>>()
            .expect("failed to parse distribution points");

        // There should be one distribution point present.
        assert_eq!(crl_distribution_points.len(), 1);
        let crl_distribution_point = crl_distribution_points
            .first()
            .expect("missing distribution point");

        // The distribution point shouldn't have revocation reasons listed.
        assert!(crl_distribution_point.reasons.is_none());

        // The distribution point shouldn't have a CRL issuer listed.
        assert!(crl_distribution_point.crl_issuer.is_none());

        // We should be able to parse the distribution point name.
        let distribution_point_name = crl_distribution_point
            .names()
            .expect("failed to parse distribution point names")
            .expect("missing distribution point name");

        // We expect the distribution point name to be a sequence of GeneralNames, not a name
        // relative to the CRL issuer.
        let names = match distribution_point_name {
            DistributionPointName::NameRelativeToCrlIssuer => {
                panic!("unexpected name relative to crl issuer")
            }
            DistributionPointName::FullName(names) => names,
        };

        // The general names should parse.
        let names = names
            .collect::<Result<Vec<_>, Error>>()
            .expect("failed to parse general names");

        // There should be one general name.
        assert_eq!(names.len(), 1);
        let name = names.first().expect("missing general name");

        // The general name should be a URI matching the expected value.
        match name {
            GeneralName::UniformResourceIdentifier(uri) => {
                assert_eq!(
                    uri.as_slice_less_safe(),
                    "http://s.symcb.com/pca3-g3.crl".as_bytes()
                );
            }
            _ => panic!("unexpected general name type"),
        }
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn test_crl_distribution_point_with_reasons() {
        let der = include_bytes!("../tests/crl_distrib_point/with_reasons.der");
        let cert =
            Cert::from_der(untrusted::Input::from(der)).expect("failed to parse certificate");

        // We expect to be able to parse the intermediate certificate's CRL distribution points.
        let crl_distribution_points = cert
            .crl_distribution_points()
            .expect("missing distribution points extension")
            .collect::<Result<Vec<_>, Error>>()
            .expect("failed to parse distribution points");

        // There should be one distribution point present.
        assert_eq!(crl_distribution_points.len(), 1);
        let crl_distribution_point = crl_distribution_points
            .first()
            .expect("missing distribution point");

        // The distribution point should include the expected revocation reasons, and no others.
        let reasons = crl_distribution_point
            .reasons
            .as_ref()
            .expect("missing revocation reasons");
        let expected = &[
            RevocationReason::KeyCompromise,
            RevocationReason::AffiliationChanged,
        ];
        for reason in RevocationReason::iter() {
            #[allow(clippy::as_conversions)]
            // revocation reason is u8, infallible to convert to usize.
            match expected.contains(&reason) {
                true => assert!(reasons.bit_set(reason as usize)),
                false => assert!(!reasons.bit_set(reason as usize)),
            }
        }
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn test_crl_distribution_point_with_crl_issuer() {
        let der = include_bytes!("../tests/crl_distrib_point/with_crl_issuer.der");
        let cert =
            Cert::from_der(untrusted::Input::from(der)).expect("failed to parse certificate");

        // We expect to be able to parse the intermediate certificate's CRL distribution points.
        let crl_distribution_points = cert
            .crl_distribution_points()
            .expect("missing distribution points extension")
            .collect::<Result<Vec<_>, Error>>()
            .expect("failed to parse distribution points");

        // There should be one distribution point present.
        assert_eq!(crl_distribution_points.len(), 1);
        let crl_distribution_point = crl_distribution_points
            .first()
            .expect("missing distribution point");

        // The CRL issuer should be present, but not anything else.
        assert!(crl_distribution_point.crl_issuer.is_some());
        assert!(crl_distribution_point.distribution_point.is_none());
        assert!(crl_distribution_point.reasons.is_none());
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn test_crl_distribution_point_bad_der() {
        // Created w/
        //   ascii2der -i tests/crl_distrib_point/unknown_tag.der.txt -o tests/crl_distrib_point/unknown_tag.der
        let der = include_bytes!("../tests/crl_distrib_point/unknown_tag.der");
        let cert =
            Cert::from_der(untrusted::Input::from(der)).expect("failed to parse certificate");

        // We expect there to be a distribution point extension, but parsing it should fail
        // due to the unknown tag in the SEQUENCE.
        let result = cert
            .crl_distribution_points()
            .expect("missing distribution points extension")
            .collect::<Result<Vec<_>, Error>>();
        assert!(matches!(result, Err(Error::BadDer)));
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn test_crl_distribution_point_only_reasons() {
        // Created w/
        //   ascii2der -i tests/crl_distrib_point/only_reasons.der.txt -o tests/crl_distrib_point/only_reasons.der
        let der = include_bytes!("../tests/crl_distrib_point/only_reasons.der");
        let cert =
            Cert::from_der(untrusted::Input::from(der)).expect("failed to parse certificate");

        // We expect there to be a distribution point extension, but parsing it should fail
        // because no distribution points or cRLIssuer are set in the SEQUENCE, just reason codes.
        let result = cert
            .crl_distribution_points()
            .expect("missing distribution points extension")
            .collect::<Result<Vec<_>, Error>>();
        assert!(matches!(result, Err(Error::MalformedExtensions)));
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn test_crl_distribution_point_name_relative_to_issuer() {
        let der = include_bytes!("../tests/crl_distrib_point/dp_name_relative_to_issuer.der");
        let cert =
            Cert::from_der(untrusted::Input::from(der)).expect("failed to parse certificate");

        // We expect to be able to parse the intermediate certificate's CRL distribution points.
        let crl_distribution_points = cert
            .crl_distribution_points()
            .expect("missing distribution points extension")
            .collect::<Result<Vec<_>, Error>>()
            .expect("failed to parse distribution points");

        // There should be one distribution point present.
        assert_eq!(crl_distribution_points.len(), 1);
        let crl_distribution_point = crl_distribution_points
            .first()
            .expect("missing distribution point");

        assert!(crl_distribution_point.crl_issuer.is_none());
        assert!(crl_distribution_point.reasons.is_none());

        // We should be able to parse the distribution point name.
        let distribution_point_name = crl_distribution_point
            .names()
            .expect("failed to parse distribution point names")
            .expect("missing distribution point name");

        // We expect the distribution point name to be a name relative to the CRL issuer.
        assert!(matches!(
            distribution_point_name,
            DistributionPointName::NameRelativeToCrlIssuer
        ));
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn test_crl_distribution_point_unknown_name_tag() {
        // Created w/
        //   ascii2der -i tests/crl_distrib_point/unknown_dp_name_tag.der.txt > tests/crl_distrib_point/unknown_dp_name_tag.der
        let der = include_bytes!("../tests/crl_distrib_point/unknown_dp_name_tag.der");
        let cert =
            Cert::from_der(untrusted::Input::from(der)).expect("failed to parse certificate");

        // We expect to be able to parse the intermediate certificate's CRL distribution points.
        let crl_distribution_points = cert
            .crl_distribution_points()
            .expect("missing distribution points extension")
            .collect::<Result<Vec<_>, Error>>()
            .expect("failed to parse distribution points");

        // There should be one distribution point present.
        assert_eq!(crl_distribution_points.len(), 1);
        let crl_distribution_point = crl_distribution_points
            .first()
            .expect("missing distribution point");

        // Parsing the distrubition point names should fail due to the unknown name tag.
        let result = crl_distribution_point.names();
        assert!(matches!(result, Err(Error::BadDer)))
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn test_crl_distribution_point_multiple() {
        let der = include_bytes!("../tests/crl_distrib_point/multiple_distribution_points.der");
        let cert =
            Cert::from_der(untrusted::Input::from(der)).expect("failed to parse certificate");

        // We expect to be able to parse the intermediate certificate's CRL distribution points.
        let crl_distribution_points = cert
            .crl_distribution_points()
            .expect("missing distribution points extension")
            .collect::<Result<Vec<_>, Error>>()
            .expect("failed to parse distribution points");

        // There should be two distribution points present.
        let (point_a, point_b) = (
            crl_distribution_points
                .first()
                .expect("missing first distribution point"),
            crl_distribution_points
                .get(1)
                .expect("missing second distribution point"),
        );

        fn get_names<'a>(
            point: &'a CrlDistributionPoint<'a>,
        ) -> impl Iterator<Item = Result<GeneralName<'a>, Error>> {
            match point
                .names()
                .expect("failed to parse distribution point names")
                .expect("missing distribution point name")
            {
                DistributionPointName::NameRelativeToCrlIssuer => {
                    panic!("unexpected relative name")
                }
                DistributionPointName::FullName(names) => names,
            }
        }

        fn uri_bytes<'a>(name: &'a GeneralName<'a>) -> &'a [u8] {
            match name {
                GeneralName::UniformResourceIdentifier(uri) => uri.as_slice_less_safe(),
                _ => panic!("unexpected name type"),
            }
        }

        // We expect to find three URIs across the two distribution points.
        let expected_names = [
            "http://example.com/crl.1.der".as_bytes(),
            "http://example.com/crl.2.der".as_bytes(),
            "http://example.com/crl.3.der".as_bytes(),
        ];
        let all_names = get_names(point_a)
            .chain(get_names(point_b))
            .collect::<Result<Vec<_>, Error>>()
            .expect("failed to parse names");

        assert_eq!(
            all_names.iter().map(uri_bytes).collect::<Vec<_>>(),
            expected_names
        );
    }
}
