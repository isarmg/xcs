#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::*;

    quickcheck! {
        fn backoff_base_first(min_ms: u64, max_ms: u64) -> TestResult {
            let min = time::Duration::from_millis(min_ms);
            let max = time::Duration::from_millis(max_ms);
            let rng = HasherRng::default();
            let mut backoff = match ExponentialBackoffMaker::new(min, max, 0.0, rng) {
                Err(_) => return TestResult::discard(),
                Ok(backoff) => backoff,
            };
            let backoff = backoff.make_backoff();

            let delay = backoff.base();
            TestResult::from_bool(min == delay)
        }

        fn backoff_base(min_ms: u64, max_ms: u64, iterations: u32) -> TestResult {
            let min = time::Duration::from_millis(min_ms);
            let max = time::Duration::from_millis(max_ms);
            let rng = HasherRng::default();
            let mut backoff = match ExponentialBackoffMaker::new(min, max, 0.0, rng) {
                Err(_) => return TestResult::discard(),
                Ok(backoff) => backoff,
            };
            let mut backoff = backoff.make_backoff();

            backoff.iterations = iterations;
            let delay = backoff.base();
            TestResult::from_bool(min <= delay && delay <= max)
        }

        fn backoff_jitter(base_ms: u64, max_ms: u64, jitter: f64) -> TestResult {
            let base = time::Duration::from_millis(base_ms);
            let max = time::Duration::from_millis(max_ms);
            let rng = HasherRng::default();
            let mut backoff = match ExponentialBackoffMaker::new(base, max, jitter, rng) {
                Err(_) => return TestResult::discard(),
                Ok(backoff) => backoff,
            };
            let mut backoff = backoff.make_backoff();

            let j = backoff.jitter(base);
            if jitter == 0.0 || base_ms == 0 || max_ms == base_ms {
                TestResult::from_bool(j == time::Duration::default())
            } else {
                TestResult::from_bool(j > time::Duration::default())
            }
        }
    }

    #[test]
    fn jitter_must_be_finite() {
        let min = time::Duration::from_millis(0);
        let max = time::Duration::from_millis(1);
        let rng = HasherRng::default();

        for n in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
            let result = ExponentialBackoffMaker::new(min, max, n, rng.clone());
            assert!(
                matches!(result, Err(InvalidBackoff(_))),
                "{} should be an invalid jitter",
                n
            );
        }
    }
}
