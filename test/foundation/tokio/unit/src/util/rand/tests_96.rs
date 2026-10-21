#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_zero_seed_from_u64() {
        let seed = RngSeed::from_u64(0);
        assert_eq!(seed.s, 0);
        assert_eq!(seed.r, 1);
    }

    #[test]
    fn non_zero_seed_from_pair() {
        let seed = RngSeed::from_pair(0, 0);
        assert_eq!(seed.s, 0);
        assert_eq!(seed.r, 1);
    }
}
