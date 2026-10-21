#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::*;

    quickcheck! {
        fn next_f64(counter: u64) -> TestResult {
            let mut rng = HasherRng {
                counter,
                ..HasherRng::default()
            };
            let n = rng.next_f64();

            TestResult::from_bool((0.0..1.0).contains(&n))
        }

        fn next_range(counter: u64, range: Range<u64>) -> TestResult {
            if range.start >= range.end{
                return TestResult::discard();
            }

            let mut rng = HasherRng {
                counter,
                ..HasherRng::default()
            };

            let n = rng.next_range(range.clone());

            TestResult::from_bool(n >= range.start && (n < range.end || range.start == range.end))
        }

        fn sample_floyd2(counter: u64, length: u64) -> TestResult {
            if !(2..=256).contains(&length) {
                return TestResult::discard();
            }

            let mut rng = HasherRng {
                counter,
                ..HasherRng::default()
            };

            let [a, b] = super::sample_floyd2(&mut rng, length);

            if a >= length || b >= length || a == b {
                return TestResult::failed();
            }

            TestResult::passed()
        }
    }

    #[test]
    fn sample_inplace_boundaries() {
        let mut r = HasherRng::default();
        match super::sample_floyd2(&mut r, 2) {
            [0, 1] | [1, 0] => (),
            array => panic!("unexpected inplace boundaries: {:?}", array),
        }
    }
}
