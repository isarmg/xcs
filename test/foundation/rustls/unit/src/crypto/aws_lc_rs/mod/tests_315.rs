#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    #[cfg(feature = "fips")]
    #[test]
    fn default_suites_are_fips() {
        assert!(
            super::DEFAULT_CIPHER_SUITES
                .iter()
                .all(|scs| scs.fips())
        );
    }

    #[cfg(not(feature = "fips"))]
    #[test]
    fn default_suites() {
        assert_eq!(super::DEFAULT_CIPHER_SUITES, super::ALL_CIPHER_SUITES);
    }

    #[test]
    fn certificate_sig_algs() {
        // `all` should not contain duplicates (not incorrect, but a waste of time)
        assert_eq!(
            super::SUPPORTED_SIG_ALGS
                .all
                .iter()
                .map(|alg| {
                    (
                        alg.public_key_alg_id()
                            .as_ref()
                            .to_vec(),
                        alg.signature_alg_id().as_ref().to_vec(),
                    )
                })
                .collect::<HashSet<_>>()
                .len(),
            super::SUPPORTED_SIG_ALGS.all.len(),
        );
    }
}
