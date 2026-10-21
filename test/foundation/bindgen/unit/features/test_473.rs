#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn release_versions_for_editions() {
        assert_eq!(
            "1.33".parse::<RustTarget>().unwrap().latest_edition(),
            RustEdition::Edition2018
        );
        assert_eq!(
            "1.56".parse::<RustTarget>().unwrap().latest_edition(),
            RustEdition::Edition2021
        );
        assert_eq!(
            "1.85".parse::<RustTarget>().unwrap().latest_edition(),
            RustEdition::Edition2024
        );
        assert_eq!(
            "nightly".parse::<RustTarget>().unwrap().latest_edition(),
            RustEdition::Edition2024
        );
    }

    #[test]
    fn target_features() {
        let features =
            RustFeatures::new_with_latest_edition(RustTarget::Stable_1_71);
        assert!(
            features.c_unwind_abi &&
                features.abi_efiapi &&
                !features.thiscall_abi
        );

        let features = RustFeatures::new(
            RustTarget::Stable_1_77,
            RustEdition::Edition2018,
        );
        assert!(!features.literal_cstr);

        let features =
            RustFeatures::new_with_latest_edition(RustTarget::Stable_1_77);
        assert!(features.literal_cstr);

        let f_nightly =
            RustFeatures::new_with_latest_edition(RustTarget::Nightly);
        assert!(
            f_nightly.vectorcall_abi &&
                f_nightly.ptr_metadata &&
                f_nightly.layout_for_ptr
        );
    }

    fn test_target(input: &str, expected: RustTarget) {
        // Two targets are equivalent if they enable the same set of features
        let expected = RustFeatures::new_with_latest_edition(expected);
        let found = RustFeatures::new_with_latest_edition(
            input.parse::<RustTarget>().unwrap(),
        );
        assert_eq!(
            expected,
            found,
            "target {input} enables features:\n{found:#?}\nand should enable features:\n{expected:#?}"
        );
    }

    fn test_invalid_target(input: &str) {
        assert!(
            input.parse::<RustTarget>().is_err(),
            "{input} should be an invalid target"
        );
    }

    #[test]
    fn valid_targets() {
        test_target("1.71", RustTarget::Stable_1_71);
        test_target("1.71.0", RustTarget::Stable_1_71);
        test_target("1.71.1", RustTarget::Stable_1_71);
        test_target("1.72", RustTarget::Stable_1_71);
        test_target("1.73", RustTarget::Stable_1_73);
        test_target("1.18446744073709551615", LATEST_STABLE_RUST);
        test_target("nightly", RustTarget::Nightly);
    }

    #[test]
    fn invalid_targets() {
        test_invalid_target("2.0");
        test_invalid_target("1.cat");
        test_invalid_target("1.0.cat");
        test_invalid_target("1.18446744073709551616");
        test_invalid_target("1.0.18446744073709551616");
        test_invalid_target("1.-1.0");
        test_invalid_target("1.0.-1");
        test_invalid_target("beta");
        test_invalid_target("1.0.0");
        test_invalid_target("1.32.0");
    }
}
