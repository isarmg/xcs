#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_seed_from_u64() {
        struct SeedableNum(u64);
        impl SeedableRng for SeedableNum {
            type Seed = [u8; 8];

            fn from_seed(seed: Self::Seed) -> Self {
                let mut x = [0u64; 1];
                le::read_u64_into(&seed, &mut x);
                SeedableNum(x[0])
            }
        }

        const N: usize = 8;
        const SEEDS: [u64; N] = [0u64, 1, 2, 3, 4, 8, 16, -1i64 as u64];
        let mut results = [0u64; N];
        for (i, seed) in SEEDS.iter().enumerate() {
            let SeedableNum(x) = SeedableNum::seed_from_u64(*seed);
            results[i] = x;
        }

        for (i1, r1) in results.iter().enumerate() {
            let weight = r1.count_ones();
            // This is the binomial distribution B(64, 0.5), so chance of
            // weight < 20 is binocdf(19, 64, 0.5) = 7.8e-4, and same for
            // weight > 44.
            assert!((20..=44).contains(&weight));

            for (i2, r2) in results.iter().enumerate() {
                if i1 == i2 {
                    continue;
                }
                let diff_weight = (r1 ^ r2).count_ones();
                assert!(diff_weight >= 20);
            }
        }

        // value-breakage test:
        assert_eq!(results[0], 5029875928683246316);
    }
}
