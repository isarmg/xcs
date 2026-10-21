#[cfg(test)]
mod test {
    use super::InvalidHistogramConfiguration;
    use crate::runtime::metrics::histogram::h2_histogram::LogHistogram;
    use crate::runtime::metrics::histogram::HistogramType;

    #[cfg(not(target_family = "wasm"))]
    mod proptests {
        use super::*;
        use crate::runtime::metrics::batch::duration_as_u64;
        use crate::runtime::metrics::histogram::h2_histogram::MAX_PRECISION;
        use proptest::prelude::*;
        use std::time::Duration;

        fn valid_log_histogram_strategy() -> impl Strategy<Value = LogHistogram> {
            (1..=50u32, 0..=MAX_PRECISION, 0..100usize).prop_map(|(n, p, bucket_offset)| {
                let p = p.min(n);
                let base = LogHistogram::from_n_p(n, p, 0);
                LogHistogram::from_n_p(n, p, bucket_offset.min(base.num_buckets - 1))
            })
        }

        fn log_histogram_settings() -> impl Strategy<Value = (u64, u64, u32)> {
            (
                duration_as_u64(Duration::from_nanos(1))..duration_as_u64(Duration::from_secs(20)),
                duration_as_u64(Duration::from_secs(1))..duration_as_u64(Duration::from_secs(1000)),
                0..MAX_PRECISION,
            )
        }

        // test against a wide assortment of different histogram configurations to ensure invariants hold
        proptest! {
            #[test]
            fn log_histogram_settings_maintain_invariants((min_value, max_value, p) in log_histogram_settings()) {
                if max_value < min_value {
                    return Ok(())
                }
                let (min_value, max_value) = (Duration::from_nanos(min_value), Duration::from_nanos(max_value));
                let histogram = LogHistogram::builder().min_value(min_value).max_value(max_value).precision_exact(p).build();
                let first_bucket_end = Duration::from_nanos(histogram.bucket_range(0).end);
                let last_bucket_start = Duration::from_nanos(histogram.bucket_range(histogram.num_buckets - 1).start);
                let second_last_bucket_start = Duration::from_nanos(histogram.bucket_range(histogram.num_buckets - 2).start);
                prop_assert!(
                    first_bucket_end  <= min_value,
                    "first bucket {first_bucket_end:?} must be less than {min_value:?}"
                );
                prop_assert!(
                    last_bucket_start > max_value,
                    "last bucket start ({last_bucket_start:?} must be at least as big as `max_value` ({max_value:?})"
                );

                // We should have the exact right number of buckets. The second to last bucket should be strictly less than max value.
                prop_assert!(
                    second_last_bucket_start < max_value,
                    "second last bucket end ({second_last_bucket_start:?} must be at least as big as `max_value` ({max_value:?})"
                );
            }

            #[test]
            fn proptest_log_histogram_invariants(histogram in valid_log_histogram_strategy()) {
                // 1. Assert that the first bucket always starts at 0
                let first_range = histogram.bucket_range(0);
                prop_assert_eq!(first_range.start, 0, "First bucket doesn't start at 0");

                // Check that bucket ranges are disjoint and contiguous
                let mut prev_end = 0;
                let mut prev_size = 0;
                for bucket in 0..histogram.num_buckets {
                    let range = histogram.bucket_range(bucket);
                    prop_assert_eq!(range.start, prev_end, "Bucket ranges are not contiguous");
                    prop_assert!(range.start < range.end, "Bucket range is empty or reversed");

                    let size = range.end - range.start;

                    // 2. Assert that the sizes of the buckets are always powers of 2
                    if bucket > 0 && bucket < histogram.num_buckets - 1 {
                        prop_assert!(size.is_power_of_two(), "Bucket size is not a power of 2");
                    }

                    if bucket > 1 {
                        // Assert that the sizes of the buckets are monotonically increasing
                        // (after the first bucket, which may be smaller than the 0 bucket)
                        prop_assert!(size >= prev_size, "Bucket sizes are not monotonically increasing: This size {size} (previous: {prev_size}). Bucket: {bucket}");
                    }


                    // 4. Assert that the size of the buckets is always within the error bound of 2^-p
                    if bucket > 0 && bucket < histogram.num_buckets - 1 {
                        let p = histogram.p as f64;
                        let error_bound = 2.0_f64.powf(-p);
                        // the most it could be wrong is by the length of the range / 2
                        let relative_error = ((size as f64 - 1.0) / 2.0) / range.start as f64;
                        prop_assert!(
                            relative_error <= error_bound,
                            "Bucket size error exceeds bound: {:?} > {:?} ({range:?})",
                            relative_error,
                            error_bound
                        );
                    }

                    prev_end = range.end;
                    prev_size = size;
                }
                prop_assert_eq!(prev_end, u64::MAX, "Last bucket should end at u64::MAX");

                // Check bijection between value_to_bucket and bucket_range
                for bucket in 0..histogram.num_buckets {
                    let range = histogram.bucket_range(bucket);
                    for value in [range.start, range.end - 1] {
                        prop_assert_eq!(
                            histogram.value_to_bucket(value),
                            bucket,
                            "value_to_bucket is not consistent with bucket_range"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn bucket_ranges_are_correct() {
        let p = 2;
        let config = HistogramType::H2(LogHistogram {
            num_buckets: 1024,
            p,
            bucket_offset: 0,
        });

        // check precise buckets. There are 2^(p+1) precise buckets
        for i in 0..2_usize.pow(p + 1) {
            assert_eq!(
                config.value_to_bucket(i as u64),
                i,
                "{i} should be in bucket {i}"
            );
        }

        let mut value = 2_usize.pow(p + 1);
        let current_bucket = value;
        while value < current_bucket * 2 {
            assert_eq!(
                config.value_to_bucket(value as u64),
                current_bucket + ((value - current_bucket) / 2),
                "bucket for {value}"
            );
            value += 1;
        }
    }

    // test buckets against known values
    #[test]
    fn bucket_computation_spot_check() {
        let p = 9;
        let config = HistogramType::H2(LogHistogram {
            num_buckets: 4096,
            p,
            bucket_offset: 0,
        });
        struct T {
            v: u64,
            bucket: usize,
        }
        let tests = [
            T { v: 1, bucket: 1 },
            T {
                v: 1023,
                bucket: 1023,
            },
            T {
                v: 1024,
                bucket: 1024,
            },
            T {
                v: 2048,
                bucket: 1536,
            },
            T {
                v: 2052,
                bucket: 1537,
            },
        ];
        for test in tests {
            assert_eq!(config.value_to_bucket(test.v), test.bucket);
        }
    }

    #[test]
    fn last_bucket_goes_to_infinity() {
        let conf = HistogramType::H2(LogHistogram::from_n_p(16, 3, 10));
        assert_eq!(conf.bucket_range(conf.num_buckets() - 1).end, u64::MAX);
    }

    #[test]
    fn bucket_offset() {
        // skip the first 10 buckets
        let conf = HistogramType::H2(LogHistogram::from_n_p(16, 3, 10));
        for i in 0..10 {
            assert_eq!(conf.value_to_bucket(i), 0);
        }
        // There are 16 1-element buckets. We skipped 10 of them. The first 2 element bucket starts
        // at 16
        assert_eq!(conf.value_to_bucket(10), 0);
        assert_eq!(conf.value_to_bucket(16), 6);
        assert_eq!(conf.value_to_bucket(17), 6);
        assert_eq!(conf.bucket_range(6), 16..18);
    }

    #[test]
    fn max_buckets_enforcement() {
        let error = LogHistogram::builder()
            .max_error(0.001)
            .max_buckets(5)
            .expect_err("this produces way more than 5 buckets");
        let num_buckets = match error {
            InvalidHistogramConfiguration::TooManyBuckets {
                required_bucket_count,
            } => required_bucket_count,
        };
        assert_eq!(num_buckets, 27291);
    }

    #[test]
    fn default_configuration_size() {
        let conf = LogHistogram::builder().build();
        assert_eq!(conf.num_buckets, 119);
    }
}
