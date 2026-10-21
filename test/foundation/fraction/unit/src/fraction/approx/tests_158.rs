#[cfg(test)]
mod tests {
    use super::Accuracy;
    use num::BigUint;

    #[test]
    fn decimal_places_uses_precomputed_multipliers() {
        assert_eq!(
            Accuracy::decimal_places(20_u8).multiplier(),
            &BigUint::from(10_u8).pow(20_u32)
        );
        assert_eq!(
            Accuracy::decimal_places(100_u8).multiplier(),
            &BigUint::from(10_u8).pow(100_u32)
        );
        assert_eq!(
            Accuracy::decimal_places(500_u16).multiplier(),
            &BigUint::from(10_u8).pow(500_u32)
        );
    }

    #[test]
    fn decimal_places_reuses_precomputed_multiplier_references() {
        assert!(std::ptr::eq(
            Accuracy::decimal_places(20_u8).multiplier(),
            Accuracy::decimal_places(20_u8).multiplier()
        ));
        assert!(std::ptr::eq(
            Accuracy::decimal_places(100_u8).multiplier(),
            Accuracy::decimal_places(100_u8).multiplier()
        ));
        assert!(std::ptr::eq(
            Accuracy::decimal_places(500_u16).multiplier(),
            Accuracy::decimal_places(500_u16).multiplier()
        ));
    }
}
