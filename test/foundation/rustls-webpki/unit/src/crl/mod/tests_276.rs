#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // redundant clone, clone_on_copy allowed to verify derived traits.
    #[allow(clippy::redundant_clone, clippy::clone_on_copy)]
    fn test_revocation_opts_builder() {
        // Trying to build a RevocationOptionsBuilder w/o CRLs should err.
        let result = RevocationOptionsBuilder::new(&[]);
        assert!(matches!(result, Err(CrlsRequired(_))));

        // The CrlsRequired error should be debug and clone when alloc is enabled.
        #[cfg(feature = "alloc")]
        {
            let err = result.unwrap_err();
            std::println!("{:?}", err.clone());
        }

        // It should be possible to build a revocation options builder with defaults.
        let crl = include_bytes!("../../tests/crls/crl.valid.der");
        let crl = BorrowedCertRevocationList::from_der(&crl[..])
            .unwrap()
            .into();
        let crls = [&crl];
        let builder = RevocationOptionsBuilder::new(&crls).unwrap();
        #[cfg(feature = "alloc")]
        {
            // The builder should be debug, and clone when alloc is enabled
            std::println!("{builder:?}");
            _ = builder.clone();
        }
        let opts = builder.build();
        assert_eq!(opts.depth, RevocationCheckDepth::Chain);
        assert_eq!(opts.status_policy, UnknownStatusPolicy::Deny);
        assert_eq!(opts.expiration_policy, ExpirationPolicy::Ignore);
        assert_eq!(opts.crls.len(), 1);

        // It should be possible to build a revocation options builder with custom depth.
        let opts = RevocationOptionsBuilder::new(&crls)
            .unwrap()
            .with_depth(RevocationCheckDepth::EndEntity)
            .build();
        assert_eq!(opts.depth, RevocationCheckDepth::EndEntity);
        assert_eq!(opts.status_policy, UnknownStatusPolicy::Deny);
        assert_eq!(opts.expiration_policy, ExpirationPolicy::Ignore);
        assert_eq!(opts.crls.len(), 1);

        // It should be possible to build a revocation options builder that allows unknown
        // revocation status.
        let opts = RevocationOptionsBuilder::new(&crls)
            .unwrap()
            .with_status_policy(UnknownStatusPolicy::Allow)
            .build();
        assert_eq!(opts.depth, RevocationCheckDepth::Chain);
        assert_eq!(opts.status_policy, UnknownStatusPolicy::Allow);
        assert_eq!(opts.expiration_policy, ExpirationPolicy::Ignore);
        assert_eq!(opts.crls.len(), 1);

        // It should be possible to specify both depth and unknown status policy together.
        let opts = RevocationOptionsBuilder::new(&crls)
            .unwrap()
            .with_status_policy(UnknownStatusPolicy::Allow)
            .with_depth(RevocationCheckDepth::EndEntity)
            .build();
        assert_eq!(opts.depth, RevocationCheckDepth::EndEntity);
        assert_eq!(opts.status_policy, UnknownStatusPolicy::Allow);
        assert_eq!(opts.expiration_policy, ExpirationPolicy::Ignore);
        assert_eq!(opts.crls.len(), 1);

        // The same should be true for explicitly forbidding unknown status.
        let opts = RevocationOptionsBuilder::new(&crls)
            .unwrap()
            .with_status_policy(UnknownStatusPolicy::Deny)
            .with_depth(RevocationCheckDepth::EndEntity)
            .build();
        assert_eq!(opts.depth, RevocationCheckDepth::EndEntity);
        assert_eq!(opts.status_policy, UnknownStatusPolicy::Deny);
        assert_eq!(opts.expiration_policy, ExpirationPolicy::Ignore);
        assert_eq!(opts.crls.len(), 1);

        // It should be possible to build a revocation options builder that allows unknown
        // revocation status.
        let opts = RevocationOptionsBuilder::new(&crls)
            .unwrap()
            .with_expiration_policy(ExpirationPolicy::Enforce)
            .build();
        assert_eq!(opts.depth, RevocationCheckDepth::Chain);
        assert_eq!(opts.status_policy, UnknownStatusPolicy::Deny);
        assert_eq!(opts.expiration_policy, ExpirationPolicy::Enforce);
        assert_eq!(opts.crls.len(), 1);

        // Built revocation options should be debug and clone when alloc is enabled.
        #[cfg(feature = "alloc")]
        {
            std::println!("{:?}", opts.clone());
        }
    }
}
