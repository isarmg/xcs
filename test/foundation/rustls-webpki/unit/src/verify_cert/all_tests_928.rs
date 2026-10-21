#[cfg(all(test, feature = "alloc", any(feature = "ring", feature = "aws-lc-rs")))]
mod tests {
    use alloc::borrow::ToOwned;
    use alloc::string::{String, ToString};
    use std::dbg;
    use std::slice;

    use super::*;
    use crate::test_utils;
    use crate::test_utils::{issuer_params, make_end_entity, make_issuer};
    use crate::trust_anchor::anchor_from_trusted_cert;
    use rcgen::{CertifiedIssuer, Issuer, KeyPair, SigningKey};

    #[test]
    fn roundtrip() {
        // 2.999.3 -> 1079.3 -> [0x84, 0x37, 0x3]
        const ENCODED: &[u8] = &[0x84, 0x37, 0x3];
        let decoded = OidDecoder::new(ENCODED);
        assert_eq!(decoded.collect::<Vec<_>>(), [2, 999, 3]);
    }

    #[test]
    fn oid_decoding() {
        assert_eq!(
            KeyUsage::server_auth().oid_values().collect::<Vec<_>>(),
            KeyUsage::SERVER_AUTH_REPR
        );
        assert_eq!(
            KeyUsage::client_auth().oid_values().collect::<Vec<_>>(),
            KeyUsage::CLIENT_AUTH_REPR
        );
    }

    #[test]
    fn eku_fail_empty() {
        let err = KeyUsage::required(EKU_SERVER_AUTH)
            .validate(KeyPurposeIdIter {
                input: &mut untrusted::Reader::new(untrusted::Input::from(&[])),
            })
            .unwrap_err();
        assert_eq!(
            err,
            Error::RequiredEkuNotFoundContext(RequiredEkuNotFoundContext {
                #[cfg(feature = "alloc")]
                required: dbg!(KeyUsage::required(EKU_SERVER_AUTH)), // Cover Debug impl
                #[cfg(feature = "alloc")]
                present: Vec::new(),
            })
        );
    }

    #[test]
    fn eku_fail_empty_with_optional() {
        let mut empty_eku = untrusted::Reader::new(untrusted::Input::from(&[]));
        let validator = KeyUsage::required_if_present(EKU_SERVER_AUTH);
        assert_eq!(
            check_eku(Some(&mut empty_eku), &validator).unwrap_err(),
            Error::EmptyEkuExtension,
        );
    }

    #[test]
    fn eku_key_purpose_id() {
        assert!(
            ExtendedKeyUsage::RequiredIfPresent(KeyPurposeId::new(EKU_SERVER_AUTH)).id()
                == KeyPurposeId::new(EKU_SERVER_AUTH)
        )
    }

    #[test]
    fn test_too_many_signatures() {
        assert!(matches!(
            build_and_verify_degenerate_chain(5, ChainTrustAnchor::NotInChain),
            ControlFlow::Break(Error::MaximumSignatureChecksExceeded)
        ));
    }

    #[test]
    fn test_too_many_path_calls() {
        assert!(matches!(
            dbg!(build_and_verify_degenerate_chain(
                10,
                ChainTrustAnchor::InChain
            )),
            ControlFlow::Break(Error::MaximumPathBuildCallsExceeded)
        ));
    }

    #[test]
    fn longest_allowed_path() {
        assert!(build_and_verify_linear_chain(1).is_ok());
        assert!(build_and_verify_linear_chain(2).is_ok());
        assert!(build_and_verify_linear_chain(3).is_ok());
        assert!(build_and_verify_linear_chain(4).is_ok());
        assert!(build_and_verify_linear_chain(5).is_ok());
        assert!(build_and_verify_linear_chain(6).is_ok());
    }

    #[test]
    fn path_too_long() {
        assert!(matches!(
            build_and_verify_linear_chain(7),
            Err(ControlFlow::Continue(Error::MaximumPathDepthExceeded))
        ));
    }

    #[test]
    fn name_constraint_budget() {
        // Issue a trust anchor that imposes name constraints. The constraint should match
        // the end entity certificate SAN.
        let mut ca_cert_params = issuer_params("Constrained Root");
        ca_cert_params.name_constraints = Some(rcgen::NameConstraints {
            permitted_subtrees: vec![rcgen::GeneralSubtree::DnsName(".com".into())],
            excluded_subtrees: vec![],
        });
        let ca_key_pair = KeyPair::generate_for(test_utils::RCGEN_SIGNATURE_ALG).unwrap();
        let ca = CertifiedIssuer::self_signed(ca_cert_params, ca_key_pair).unwrap();

        // Create a series of intermediate issuers. We'll only use one in the actual built path,
        // helping demonstrate that the name constraint budget is not expended checking certificates
        // that are not part of the path we compute.
        let mut intermediates = Vec::with_capacity(5);
        for i in 0..5 {
            let intermediate = issuer_params(format!("Intermediate {i}"));
            let intermediate_key_pair =
                KeyPair::generate_for(test_utils::RCGEN_SIGNATURE_ALG).unwrap();
            // Each intermediate should be issued by the trust anchor.
            intermediates.push(
                CertifiedIssuer::signed_by(intermediate, intermediate_key_pair, &ca).unwrap(),
            );
        }

        // Create an end-entity cert that is issued by the last of the intermediates.
        let ee_cert = make_end_entity(intermediates.last().unwrap());
        let ee_cert = EndEntityCert::try_from(ee_cert.cert.der()).unwrap();

        // We use a custom budget to make it easier to write a test, otherwise it is tricky to
        // stuff enough names/constraints into the potential chains while staying within the path
        // depth limit and the build chain call limit.
        let passing_budget = Budget {
            // One comparison against the intermediate's distinguished name.
            // One comparison against the EE's distinguished name.
            // One comparison against the EE's SAN.
            //  = 3 total comparisons.
            name_constraint_comparisons: 3,
            ..Budget::default()
        };

        let anchors = &[anchor_from_trusted_cert(ca.der()).unwrap()];
        let intermediates_der = intermediates
            .iter()
            .map(|issuer| issuer.der().clone())
            .collect::<Vec<_>>();

        // Validation should succeed with the name constraint comparison budget allocated above.
        // This shows that we're not consuming budget on unused intermediates: we didn't budget
        // enough comparisons for that to pass the overall chain building.
        let path = verify_chain(
            anchors,
            &intermediates_der,
            &ee_cert,
            None,
            Some(passing_budget),
        )
        .unwrap();
        assert_eq!(path.anchor().subject, anchors.first().unwrap().subject);

        let failing_budget = Budget {
            // See passing_budget: 2 comparisons is not sufficient.
            name_constraint_comparisons: 2,
            ..Budget::default()
        };
        // Validation should fail when the budget is smaller than the number of comparisons performed
        // on the validated path. This demonstrates we properly fail path building when too many
        // name constraint comparisons occur.
        let result = verify_chain(
            anchors,
            &intermediates_der,
            &ee_cert,
            None,
            Some(failing_budget),
        );

        assert!(matches!(
            result,
            Err(ControlFlow::Break(
                Error::MaximumNameConstraintComparisonsExceeded
            ))
        ));
    }

    /// This test builds a PKI like the following diagram depicts. We first verify
    /// that we can build a path EE -> B -> A -> TA. Next we supply a custom path verification
    /// function that rejects the B->A path, and verify that we build a path EE -> B -> C -> TA.
    ///
    ///        ┌───────────┐
    ///        │           │
    ///        │     TA    │
    ///        │           │
    ///        └───┬───┬───┘
    ///            │   │
    ///            │   │
    /// ┌────────┐◄┘   └──►┌────────┐
    /// │        │         │        │
    /// │   A    │         │   C    │
    /// │        │         │        │
    /// └────┬───┘         └───┬────┘
    ///      │                 │
    ///      │                 │
    ///      │   ┌─────────┐   │
    ///      └──►│         │◄──┘
    ///          │    B    │
    ///          │         │
    ///          └────┬────┘
    ///               │
    ///               │
    ///               │
    ///          ┌────▼────┐
    ///          │         │
    ///          │    EE   │
    ///          │         │
    ///          └─────────┘
    #[test]
    fn test_reject_candidate_path() {
        // Create a trust anchor, and use it to issue two distinct intermediate certificates, each
        // with a unique subject and keypair.
        let trust_anchor = make_issuer("Trust Anchor");
        let trust_anchor_cert = Cert::from_der(untrusted::Input::from(trust_anchor.der())).unwrap();
        let trust_anchors = &[anchor_from_trusted_cert(trust_anchor.der()).unwrap()];

        let intermediate_a = make_intermediate("Intermediate A", &trust_anchor);
        let intermediate_c = make_intermediate("Intermediate C", &trust_anchor);

        // Next, create an intermediate that is issued by both of the intermediates above.
        // Both should share the same subject, and key pair, but will differ in the issuer.
        let (intermediate_b, intermediate_b_a_cert, intermediate_b_c_cert) = {
            let key = KeyPair::generate_for(test_utils::RCGEN_SIGNATURE_ALG).unwrap();
            let params = issuer_params("Intermediate");
            let intermediate_b_a_cert = params.signed_by(&key, &intermediate_a).unwrap();
            let intermediate_b_c_cert = params.signed_by(&key, &intermediate_c).unwrap();
            let issuer = Issuer::new(params, key);
            (issuer, intermediate_b_a_cert, intermediate_b_c_cert)
        };

        let intermediates = &[
            intermediate_a.der().clone(),
            intermediate_c.der().clone(),
            intermediate_b_a_cert.der().clone(),
            intermediate_b_c_cert.der().clone(),
        ];

        // Create an end entity certificate signed by the keypair of the intermediates created above.
        let ee = make_end_entity(&intermediate_b);
        let ee_cert = &EndEntityCert::try_from(ee.cert.der()).unwrap();

        // We should be able to create a valid path from EE to trust anchor.
        let path = verify_chain(trust_anchors, intermediates, ee_cert, None, None).unwrap();
        let path_intermediates = path.intermediate_certificates().collect::<Vec<_>>();

        // We expect that without applying any additional constraints, that the path will be
        // EE -> intermediate_b_a -> intermediate_a -> trust_anchor.
        let intermediate_a_cert =
            Cert::from_der(untrusted::Input::from(intermediate_a.der())).unwrap();
        assert_eq!(path_intermediates.len(), 2);
        assert_eq!(
            path_intermediates[0].issuer(),
            intermediate_a_cert.subject()
        );
        assert_eq!(path_intermediates[1].issuer(), trust_anchor_cert.subject());

        // Now, we'll create a function that will reject the intermediate_b_a path.
        let expected_chain = |path: &VerifiedPath<'_>| {
            for intermediate in path.intermediate_certificates() {
                // Reject any intermediates issued by intermediate A.
                if intermediate.issuer() == intermediate_a_cert.subject() {
                    return Err(Error::UnknownIssuer);
                }
            }

            Ok(())
        };

        // We should still be able to build a valid path.
        let path = verify_chain(
            trust_anchors,
            intermediates,
            ee_cert,
            Some(&expected_chain),
            None,
        )
        .unwrap();
        let path_intermediates = path.intermediate_certificates().collect::<Vec<_>>();

        // We expect that the path will now be
        // EE -> intermediate_b_c -> intermediate_c -> trust_anchor.
        let intermediate_c_cert =
            Cert::from_der(untrusted::Input::from(intermediate_c.der())).unwrap();
        assert_eq!(path_intermediates.len(), 2);
        assert_eq!(
            path_intermediates[0].issuer(),
            intermediate_c_cert.subject()
        );
        assert_eq!(path_intermediates[1].issuer(), trust_anchor_cert.subject());
    }

    #[test]
    fn distrusted_spki() {
        fn verify(
            ee_cert: &EndEntityCert<'_>,
            intermediates: &[CertificateDer<'_>],
            trust_anchors: &[TrustAnchor<'_>],
            distrusted_spkis: &[&[u8]],
        ) -> Result<usize, Error> {
            use core::time::Duration;

            ChainOptions {
                eku: KeyUsage::server_auth(),
                supported_sig_algs: crate::ALL_VERIFICATION_ALGS,
                trust_anchors,
                intermediate_certs: intermediates,
                revocation: None,
                distrusted_spkis,
            }
            .build_chain(
                ee_cert,
                UnixTime::since_unix_epoch(Duration::from_secs(0x1fed_f00d)),
                None,
            )
            .map(|path| path.intermediate_certificates().count())
        }

        let root = make_issuer("Root");
        let trust_anchors = &[anchor_from_trusted_cert(root.der()).unwrap()];
        let issuing_intermediate = make_intermediate("Issuing Intermediate", &root);
        let unrelated = make_intermediate("Intermediate", &root);

        let params = issuer_params("Intermediate");
        let key = KeyPair::generate_for(test_utils::RCGEN_SIGNATURE_ALG).unwrap();
        // Issue the same SPKI under two different issuers to test cross-signing.
        let intermediate = params.signed_by(&key, &issuing_intermediate).unwrap();
        let cross_signed = params.signed_by(&key, &root).unwrap();
        let ee = make_end_entity(&Issuer::new(params, key));
        let ee_cert = EndEntityCert::try_from(ee.cert.der()).unwrap();
        let intermediates = &[
            unrelated.der().clone(),
            intermediate.der().clone(),
            issuing_intermediate.der().clone(),
        ];
        assert_eq!(
            verify(&ee_cert, intermediates, trust_anchors, DISTRUSTED_SPKIS),
            Ok(2)
        );

        // Distrusting any certificate in the path, including the root or leaf, rejects the chain.
        for cert in [
            root.der(),
            issuing_intermediate.der(),
            intermediate.der(),
            ee.cert.der(),
        ] {
            assert_eq!(
                verify(
                    &ee_cert,
                    intermediates,
                    trust_anchors,
                    &[anchor_from_trusted_cert(cert)
                        .unwrap()
                        .subject_public_key_info
                        .as_ref()]
                ),
                Err(Error::CertDistrusted)
            );
        }

        // Distrusting an unused intermediate leaves the valid path usable.
        assert_eq!(
            verify(
                &ee_cert,
                intermediates,
                trust_anchors,
                &[anchor_from_trusted_cert(unrelated.der())
                    .unwrap()
                    .subject_public_key_info
                    .as_ref()]
            ),
            Ok(2)
        );

        // A different cross-sign provides a valid path around the distrusted issuer.
        let intermediates = &[
            intermediate.der().clone(),
            issuing_intermediate.der().clone(),
            cross_signed.der().clone(),
        ];
        let issuer_spki = anchor_from_trusted_cert(issuing_intermediate.der()).unwrap();
        assert_eq!(
            verify(
                &ee_cert,
                intermediates,
                trust_anchors,
                &[issuer_spki.subject_public_key_info.as_ref()]
            ),
            Ok(1)
        );

        // Distrusting the shared SPKI rejects the original certificate and its cross-sign.
        let intermediate_spki = anchor_from_trusted_cert(intermediate.der()).unwrap();
        assert_eq!(
            verify(
                &ee_cert,
                intermediates,
                trust_anchors,
                &[intermediate_spki.subject_public_key_info.as_ref()]
            ),
            Err(Error::CertDistrusted)
        );

        // Reject a distrusted intermediate before searching for its issuer.
        assert_eq!(
            verify(
                &ee_cert,
                &[intermediate.der().clone()],
                &[],
                &[intermediate_spki.subject_public_key_info.as_ref()]
            ),
            Err(Error::CertDistrusted)
        );
    }

    fn make_intermediate(
        org_name: impl Into<String>,
        issuer: &Issuer<'_, impl SigningKey>,
    ) -> CertifiedIssuer<'static, KeyPair> {
        let params = issuer_params(org_name);
        let key = KeyPair::generate_for(test_utils::RCGEN_SIGNATURE_ALG).unwrap();
        CertifiedIssuer::signed_by(params, key, issuer).unwrap()
    }

    fn build_and_verify_degenerate_chain(
        intermediate_count: usize,
        trust_anchor: ChainTrustAnchor,
    ) -> ControlFlow<Error, Error> {
        let ca = make_issuer("Bogus Subject");
        let mut intermediate_chain = IntermediateChain::new(intermediate_count, true, &ca);

        let verify_trust_anchor = match trust_anchor {
            ChainTrustAnchor::InChain => make_issuer("Bogus Trust Anchor"),
            ChainTrustAnchor::NotInChain => ca,
        };

        let ee_cert = make_end_entity(&intermediate_chain.last_issuer);
        let ee_cert = EndEntityCert::try_from(ee_cert.cert.der()).unwrap();
        let webpki_ta = anchor_from_trusted_cert(verify_trust_anchor.der()).unwrap();
        if matches!(trust_anchor, ChainTrustAnchor::InChain) {
            // Note: we clone the trust anchor DER here because we can't move it into the chain
            // as it's loaned to webpki_ta above.
            intermediate_chain
                .chain
                .insert(0, verify_trust_anchor.der().to_owned())
        }

        verify_chain(
            &[webpki_ta],
            &intermediate_chain.chain,
            &ee_cert,
            None,
            None,
        )
        .map(|_| ())
        .unwrap_err()
    }

    #[cfg(feature = "alloc")]
    enum ChainTrustAnchor {
        NotInChain,
        InChain,
    }

    fn build_and_verify_linear_chain(chain_length: usize) -> Result<(), ControlFlow<Error, Error>> {
        let ca = make_issuer(format!("Bogus Subject {chain_length}"));
        let intermediate_chain = IntermediateChain::new(chain_length, false, &ca);

        let anchor = anchor_from_trusted_cert(ca.der()).unwrap();
        let anchors = slice::from_ref(&anchor);

        let ee_cert = make_end_entity(&intermediate_chain.last_issuer);
        let ee_cert = EndEntityCert::try_from(ee_cert.cert.der()).unwrap();

        let expected_chain = |path: &VerifiedPath<'_>| {
            assert_eq!(path.anchor().subject, anchor.subject);
            assert!(public_values_eq(path.end_entity().subject, ee_cert.subject));
            assert_eq!(path.intermediate_certificates().count(), chain_length);

            let intermediate_certs = intermediate_chain
                .chain
                .iter()
                .map(|der| Cert::from_der(untrusted::Input::from(der)).unwrap())
                .collect::<Vec<_>>();

            for (cert, expected) in path
                .intermediate_certificates()
                .rev()
                .zip(intermediate_certs.iter())
            {
                assert!(public_values_eq(cert.subject, expected.subject));
                assert_eq!(cert.der(), expected.der());
            }

            for (cert, expected) in path
                .intermediate_certificates()
                .zip(intermediate_certs.iter().rev())
            {
                assert!(public_values_eq(cert.subject, expected.subject));
                assert_eq!(cert.der(), expected.der());
            }

            Ok(())
        };

        verify_chain(
            anchors,
            &intermediate_chain.chain,
            &ee_cert,
            Some(&expected_chain),
            None,
        )
        .map(|_| ())
    }

    struct IntermediateChain {
        last_issuer: Issuer<'static, KeyPair>,
        chain: Vec<CertificateDer<'static>>,
    }

    impl IntermediateChain {
        fn new(chain_length: usize, all_same_subject: bool, ca_cert: &Issuer<'_, KeyPair>) -> Self {
            let mut chain = Vec::with_capacity(chain_length);

            let mut prev = None;
            for i in 0..chain_length {
                let issuer = match &prev {
                    Some(prev) => prev,
                    None => ca_cert,
                };

                let intermediate = issuer_params(match all_same_subject {
                    true => "Bogus Subject".to_string(),
                    false => format!("Bogus Subject {i}"),
                });

                let key_pair = KeyPair::generate_for(test_utils::RCGEN_SIGNATURE_ALG).unwrap();
                let cert = intermediate.signed_by(&key_pair, issuer).unwrap();

                chain.push(cert.der().clone());
                prev = Some(Issuer::new(intermediate, key_pair));
            }

            Self {
                last_issuer: prev.unwrap(),
                chain,
            }
        }
    }

    fn verify_chain<'a>(
        trust_anchors: &'a [TrustAnchor<'a>],
        intermediate_certs: &'a [CertificateDer<'a>],
        ee_cert: &'a EndEntityCert<'a>,
        verify_path: Option<&dyn Fn(&VerifiedPath<'_>) -> Result<(), Error>>,
        budget: Option<Budget>,
    ) -> Result<VerifiedPath<'a>, ControlFlow<Error, Error>> {
        use core::time::Duration;

        let time = UnixTime::since_unix_epoch(Duration::from_secs(0x1fed_f00d));
        let mut path = PartialPath::new(ee_cert);
        let opts = ChainOptions {
            eku: KeyUsage::server_auth(),
            supported_sig_algs: crate::ALL_VERIFICATION_ALGS,
            trust_anchors,
            intermediate_certs,
            revocation: None,
            distrusted_spkis: DISTRUSTED_SPKIS,
        };

        match opts.build_chain_inner(
            &mut path,
            time,
            verify_path,
            0,
            &mut budget.unwrap_or_default(),
        ) {
            Ok(anchor) => Ok(VerifiedPath::new(ee_cert, anchor, path)),
            Err(err) => Err(err),
        }
    }
}
