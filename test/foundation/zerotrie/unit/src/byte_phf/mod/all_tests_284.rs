#[cfg(all(test, feature = "alloc"))]
mod tests {
    use super::*;
    use alloc::vec::Vec;
    extern crate std;

    fn random_alphanums(seed: u64, len: usize) -> Vec<u8> {
        use rand::seq::SliceRandom;
        use rand::SeedableRng;

        let mut bytes: Vec<u8> =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789".into();
        let mut rng = rand_pcg::Lcg64Xsh32::seed_from_u64(seed);
        bytes.partial_shuffle(&mut rng, len).0.into()
    }

    #[test]
    fn test_smaller() {
        let mut count_by_p = [0; 256];
        let mut count_by_qmax = [0; 256];
        for len in 1..16 {
            for seed in 0..150 {
                let keys = random_alphanums(seed, len);
                let keys_str = core::str::from_utf8(&keys).unwrap();
                let computed = PerfectByteHashMap::try_new(&keys).expect(keys_str);
                computed
                    .check()
                    .unwrap_or_else(|_| panic!("{}", std::str::from_utf8(&keys).expect(keys_str)));
                let (p, qmax) = computed.p_qmax().unwrap();
                count_by_p[p as usize] += 1;
                count_by_qmax[qmax as usize] += 1;
            }
        }
        std::println!("count_by_p (smaller): {count_by_p:?}");
        std::println!("count_by_qmax (smaller): {count_by_qmax:?}");
        let count_fastq = count_by_qmax[0..=Q_FAST_MAX as usize].iter().sum::<usize>();
        let count_slowq = count_by_qmax[Q_FAST_MAX as usize + 1..]
            .iter()
            .sum::<usize>();
        std::println!("fastq/slowq: {count_fastq}/{count_slowq}");
        // Assert that 99% of cases resolve to the fast hash
        assert!(count_fastq >= count_slowq * 100);
    }

    #[test]
    fn test_larger() {
        let mut count_by_p = [0; 256];
        let mut count_by_qmax = [0; 256];
        for len in 16..60 {
            for seed in 0..75 {
                let keys = random_alphanums(seed, len);
                let keys_str = core::str::from_utf8(&keys).unwrap();
                let computed = PerfectByteHashMap::try_new(&keys).expect(keys_str);
                computed
                    .check()
                    .unwrap_or_else(|_| panic!("{}", std::str::from_utf8(&keys).expect(keys_str)));
                let (p, qmax) = computed.p_qmax().unwrap();
                count_by_p[p as usize] += 1;
                count_by_qmax[qmax as usize] += 1;
            }
        }
        std::println!("count_by_p (larger): {count_by_p:?}");
        std::println!("count_by_qmax (larger): {count_by_qmax:?}");
        let count_fastq = count_by_qmax[0..=Q_FAST_MAX as usize].iter().sum::<usize>();
        let count_slowq = count_by_qmax[Q_FAST_MAX as usize + 1..]
            .iter()
            .sum::<usize>();
        std::println!("fastq/slowq: {count_fastq}/{count_slowq}");
        // Assert that 99% of cases resolve to the fast hash
        assert!(count_fastq >= count_slowq * 100);
    }

    #[test]
    fn test_hard_cases() {
        let keys = [
            0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
            46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67,
            68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89,
            90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108,
            109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125,
            126, 195, 196,
        ];

        let computed = PerfectByteHashMap::try_new(&keys).unwrap();
        let (p, qmax) = computed.p_qmax().unwrap();
        assert_eq!(p, 69);
        assert_eq!(qmax, 67);
    }

    #[test]
    fn test_build_read_small() {
        #[derive(Debug)]
        struct TestCase<'a> {
            keys: &'a str,
            expected: &'a [u8],
            reordered_keys: &'a str,
        }
        let cases = [
            TestCase {
                keys: "ab",
                expected: &[0, 0, 0, b'b', b'a'],
                reordered_keys: "ba",
            },
            TestCase {
                keys: "abc",
                expected: &[0, 0, 0, 0, b'c', b'a', b'b'],
                reordered_keys: "cab",
            },
            TestCase {
                // Note: splitting "a" and "c" into different buckets requires the heavier hash
                // function because the difference between "a" and "c" is the period (2).
                keys: "ac",
                expected: &[1, 0, 1, b'c', b'a'],
                reordered_keys: "ca",
            },
            TestCase {
                keys: "aceg",
                expected: &[1, 0, 0, 1, 1, b'e', b'a', b'c', b'g'],
                reordered_keys: "eacg",
            },
            TestCase {
                keys: "abd",
                expected: &[0, 0, 1, 3, b'a', b'b', b'd'],
                reordered_keys: "abd",
            },
            TestCase {
                keys: "def",
                expected: &[0, 0, 0, 0, b'f', b'd', b'e'],
                reordered_keys: "fde",
            },
            TestCase {
                keys: "fi",
                expected: &[0, 0, 0, b'f', b'i'],
                reordered_keys: "fi",
            },
            TestCase {
                keys: "gh",
                expected: &[0, 0, 0, b'h', b'g'],
                reordered_keys: "hg",
            },
            TestCase {
                keys: "lm",
                expected: &[0, 0, 0, b'l', b'm'],
                reordered_keys: "lm",
            },
            TestCase {
                // Note: "a" and "q" (0x61 and 0x71) are very hard to split; only a handful of
                // hash function crates can get them into separate buckets.
                keys: "aq",
                expected: &[4, 0, 1, b'a', b'q'],
                reordered_keys: "aq",
            },
            TestCase {
                keys: "xy",
                expected: &[0, 0, 0, b'x', b'y'],
                reordered_keys: "xy",
            },
            TestCase {
                keys: "xyz",
                expected: &[0, 0, 0, 0, b'x', b'y', b'z'],
                reordered_keys: "xyz",
            },
            TestCase {
                keys: "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
                expected: &[
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 10, 12, 16, 4, 4, 4, 4, 4, 4, 8, 4, 4, 4, 16,
                    16, 16, 16, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
                    2, 0, 7, 104, 105, 106, 107, 108, 109, 110, 111, 112, 117, 118, 119, 68, 69,
                    70, 113, 114, 65, 66, 67, 120, 121, 122, 115, 72, 73, 74, 71, 80, 81, 82, 83,
                    84, 85, 86, 87, 88, 89, 90, 75, 76, 77, 78, 79, 103, 97, 98, 99, 116, 100, 102,
                    101,
                ],
                reordered_keys: "hijklmnopuvwDEFqrABCxyzsHIJGPQRSTUVWXYZKLMNOgabctdfe",
            },
            TestCase {
                keys: "abcdefghij",
                expected: &[
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100, 101, 102, 103, 104, 105, 106, 97, 98, 99,
                ],
                reordered_keys: "defghijabc",
            },
            TestCase {
                // This is a small case that resolves to the slow hasher
                keys: "Jbej",
                expected: &[2, 0, 0, 102, 0, b'j', b'e', b'b', b'J'],
                reordered_keys: "jebJ",
            },
            TestCase {
                // This is another small case that resolves to the slow hasher
                keys: "JFNv",
                expected: &[1, 98, 0, 2, 0, b'J', b'F', b'N', b'v'],
                reordered_keys: "JFNv",
            },
        ];
        for cas in cases {
            let computed = PerfectByteHashMap::try_new(cas.keys.as_bytes()).expect(cas.keys);
            assert_eq!(computed.as_bytes(), cas.expected, "{cas:?}");
            assert_eq!(computed.keys(), cas.reordered_keys.as_bytes(), "{cas:?}");
            computed.check().expect(cas.keys);
        }
    }
}
