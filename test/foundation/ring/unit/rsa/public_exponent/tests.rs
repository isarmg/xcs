#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_public_exponent_constants() {
        for value in PublicExponent::ALL_CONSTANTS.iter() {
            let value: u64 = value.0.into();
            assert_eq!(value & 1, 1);
            assert!(value >= PublicExponent::_3.0.into()); // The absolute minimum.
            assert!(value <= PublicExponent::MAX.0.into());
        }
    }
}
