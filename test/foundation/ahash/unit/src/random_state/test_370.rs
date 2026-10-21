#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_unique() {
        let a = RandomState::generate_with(1, 2, 3, 4);
        let b = RandomState::generate_with(1, 2, 3, 4);
        assert_ne!(a.build_hasher().finish(), b.build_hasher().finish());
    }

    #[cfg(all(feature = "runtime-rng"))]
    #[test]
    fn test_not_pi() {
        assert_ne!(PI, get_fixed_seeds()[0]);
    }

    #[cfg(all(not (feature = "runtime-rng")))]
    #[test]
    fn test_pi() {
        assert_eq!(PI, get_fixed_seeds()[0]);
    }

    #[test]
    fn test_with_seeds_const() {
        const _CONST_RANDOM_STATE: RandomState = RandomState::with_seeds(17, 19, 21, 23);
    }
}
