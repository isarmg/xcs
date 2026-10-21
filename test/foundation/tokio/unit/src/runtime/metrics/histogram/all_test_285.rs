#[cfg(all(test, target_has_atomic = "64"))]
mod test {
    use super::*;

    macro_rules! assert_bucket_eq {
        ($h:expr, $bucket:expr, $val:expr) => {{
            assert_eq!($h.buckets[$bucket], $val);
        }};
    }

    fn linear(resolution: u64, num_buckets: usize) -> Histogram {
        HistogramBuilder {
            histogram_type: HistogramType::Linear(LinearHistogram {
                bucket_width: resolution,
                num_buckets,
            }),
            legacy: None,
        }
        .build()
    }

    #[test]
    fn test_legacy_builder() {
        let mut builder = HistogramBuilder::new();
        builder.legacy_mut(|b| b.num_buckets = 20);
        assert_eq!(builder.build().num_buckets(), 20);
    }

    #[test]
    fn log_scale_resolution_1() {
        let h = HistogramBuilder {
            histogram_type: HistogramType::LogLegacy(LegacyLogHistogram {
                first_bucket_width: 1,
                num_buckets: 10,
            }),
            legacy: None,
        }
        .build();

        assert_eq!(h.bucket_range(0), 0..1);
        assert_eq!(h.bucket_range(1), 1..2);
        assert_eq!(h.bucket_range(2), 2..4);
        assert_eq!(h.bucket_range(3), 4..8);
        assert_eq!(h.bucket_range(9), 256..u64::MAX);

        let mut b = HistogramBatch::from_histogram(&h);

        b.measure(0, 1);
        assert_bucket_eq!(b, 0, 1);
        assert_bucket_eq!(b, 1, 0);

        b.measure(1, 1);
        assert_bucket_eq!(b, 0, 1);
        assert_bucket_eq!(b, 1, 1);
        assert_bucket_eq!(b, 2, 0);

        b.measure(2, 1);
        assert_bucket_eq!(b, 0, 1);
        assert_bucket_eq!(b, 1, 1);
        assert_bucket_eq!(b, 2, 1);

        b.measure(3, 1);
        assert_bucket_eq!(b, 0, 1);
        assert_bucket_eq!(b, 1, 1);
        assert_bucket_eq!(b, 2, 2);

        b.measure(4, 1);
        assert_bucket_eq!(b, 0, 1);
        assert_bucket_eq!(b, 1, 1);
        assert_bucket_eq!(b, 2, 2);
        assert_bucket_eq!(b, 3, 1);

        b.measure(100, 1);
        assert_bucket_eq!(b, 7, 1);

        b.measure(128, 1);
        assert_bucket_eq!(b, 8, 1);

        b.measure(4096, 1);
        assert_bucket_eq!(b, 9, 1);

        b.measure(u64::MAX, 1);
        assert_bucket_eq!(b, 9, 2);
    }

    #[test]
    fn log_scale_resolution_2() {
        let h = HistogramBuilder {
            histogram_type: HistogramType::LogLegacy(LegacyLogHistogram {
                num_buckets: 10,
                first_bucket_width: 2,
            }),
            legacy: None,
        }
        .build();

        assert_eq!(h.bucket_range(0), 0..2);
        assert_eq!(h.bucket_range(1), 2..4);
        assert_eq!(h.bucket_range(2), 4..8);
        assert_eq!(h.bucket_range(3), 8..16);
        assert_eq!(h.bucket_range(9), 512..u64::MAX);

        let mut b = HistogramBatch::from_histogram(&h);

        b.measure(0, 1);
        assert_bucket_eq!(b, 0, 1);
        assert_bucket_eq!(b, 1, 0);

        b.measure(1, 1);
        assert_bucket_eq!(b, 0, 2);
        assert_bucket_eq!(b, 1, 0);

        b.measure(2, 1);
        assert_bucket_eq!(b, 0, 2);
        assert_bucket_eq!(b, 1, 1);
        assert_bucket_eq!(b, 2, 0);

        b.measure(3, 1);
        assert_bucket_eq!(b, 0, 2);
        assert_bucket_eq!(b, 1, 2);
        assert_bucket_eq!(b, 2, 0);

        b.measure(4, 1);
        assert_bucket_eq!(b, 0, 2);
        assert_bucket_eq!(b, 1, 2);
        assert_bucket_eq!(b, 2, 1);

        b.measure(5, 1);
        assert_bucket_eq!(b, 0, 2);
        assert_bucket_eq!(b, 1, 2);
        assert_bucket_eq!(b, 2, 2);

        b.measure(6, 1);
        assert_bucket_eq!(b, 0, 2);
        assert_bucket_eq!(b, 1, 2);
        assert_bucket_eq!(b, 2, 3);

        b.measure(7, 1);
        assert_bucket_eq!(b, 0, 2);
        assert_bucket_eq!(b, 1, 2);
        assert_bucket_eq!(b, 2, 4);

        b.measure(8, 1);
        assert_bucket_eq!(b, 0, 2);
        assert_bucket_eq!(b, 1, 2);
        assert_bucket_eq!(b, 2, 4);
        assert_bucket_eq!(b, 3, 1);

        b.measure(100, 1);
        assert_bucket_eq!(b, 6, 1);

        b.measure(128, 1);
        assert_bucket_eq!(b, 7, 1);

        b.measure(4096, 1);
        assert_bucket_eq!(b, 9, 1);

        for bucket in h.buckets.iter() {
            assert_eq!(bucket.load(Relaxed), 0);
        }

        b.submit(&h);

        for i in 0..h.buckets.len() {
            assert_eq!(h.buckets[i].load(Relaxed), b.buckets[i]);
        }

        b.submit(&h);

        for i in 0..h.buckets.len() {
            assert_eq!(h.buckets[i].load(Relaxed), b.buckets[i]);
        }
    }

    #[test]
    fn linear_scale_resolution_1() {
        let h = linear(1, 10);

        assert_eq!(h.bucket_range(0), 0..1);
        assert_eq!(h.bucket_range(1), 1..2);
        assert_eq!(h.bucket_range(2), 2..3);
        assert_eq!(h.bucket_range(3), 3..4);
        assert_eq!(h.bucket_range(9), 9..u64::MAX);

        let mut b = HistogramBatch::from_histogram(&h);

        b.measure(0, 1);
        assert_bucket_eq!(b, 0, 1);
        assert_bucket_eq!(b, 1, 0);

        b.measure(1, 1);
        assert_bucket_eq!(b, 0, 1);
        assert_bucket_eq!(b, 1, 1);
        assert_bucket_eq!(b, 2, 0);

        b.measure(2, 1);
        assert_bucket_eq!(b, 0, 1);
        assert_bucket_eq!(b, 1, 1);
        assert_bucket_eq!(b, 2, 1);
        assert_bucket_eq!(b, 3, 0);

        b.measure(3, 1);
        assert_bucket_eq!(b, 0, 1);
        assert_bucket_eq!(b, 1, 1);
        assert_bucket_eq!(b, 2, 1);
        assert_bucket_eq!(b, 3, 1);

        b.measure(5, 1);
        assert_bucket_eq!(b, 5, 1);

        b.measure(4096, 1);
        assert_bucket_eq!(b, 9, 1);

        for bucket in h.buckets.iter() {
            assert_eq!(bucket.load(Relaxed), 0);
        }

        b.submit(&h);

        for i in 0..h.buckets.len() {
            assert_eq!(h.buckets[i].load(Relaxed), b.buckets[i]);
        }

        b.submit(&h);

        for i in 0..h.buckets.len() {
            assert_eq!(h.buckets[i].load(Relaxed), b.buckets[i]);
        }
    }

    #[test]
    fn linear_scale_resolution_100() {
        let h = linear(100, 10);

        assert_eq!(h.bucket_range(0), 0..100);
        assert_eq!(h.bucket_range(1), 100..200);
        assert_eq!(h.bucket_range(2), 200..300);
        assert_eq!(h.bucket_range(3), 300..400);
        assert_eq!(h.bucket_range(9), 900..u64::MAX);

        let mut b = HistogramBatch::from_histogram(&h);

        b.measure(0, 1);
        assert_bucket_eq!(b, 0, 1);
        assert_bucket_eq!(b, 1, 0);

        b.measure(50, 1);
        assert_bucket_eq!(b, 0, 2);
        assert_bucket_eq!(b, 1, 0);

        b.measure(100, 1);
        assert_bucket_eq!(b, 0, 2);
        assert_bucket_eq!(b, 1, 1);
        assert_bucket_eq!(b, 2, 0);

        b.measure(101, 1);
        assert_bucket_eq!(b, 0, 2);
        assert_bucket_eq!(b, 1, 2);
        assert_bucket_eq!(b, 2, 0);

        b.measure(200, 1);
        assert_bucket_eq!(b, 0, 2);
        assert_bucket_eq!(b, 1, 2);
        assert_bucket_eq!(b, 2, 1);

        b.measure(299, 1);
        assert_bucket_eq!(b, 0, 2);
        assert_bucket_eq!(b, 1, 2);
        assert_bucket_eq!(b, 2, 2);

        b.measure(222, 1);
        assert_bucket_eq!(b, 0, 2);
        assert_bucket_eq!(b, 1, 2);
        assert_bucket_eq!(b, 2, 3);

        b.measure(300, 1);
        assert_bucket_eq!(b, 0, 2);
        assert_bucket_eq!(b, 1, 2);
        assert_bucket_eq!(b, 2, 3);
        assert_bucket_eq!(b, 3, 1);

        b.measure(888, 1);
        assert_bucket_eq!(b, 8, 1);

        b.measure(4096, 1);
        assert_bucket_eq!(b, 9, 1);

        for bucket in h.buckets.iter() {
            assert_eq!(bucket.load(Relaxed), 0);
        }

        b.submit(&h);

        for i in 0..h.buckets.len() {
            assert_eq!(h.buckets[i].load(Relaxed), b.buckets[i]);
        }

        b.submit(&h);

        for i in 0..h.buckets.len() {
            assert_eq!(h.buckets[i].load(Relaxed), b.buckets[i]);
        }
    }

    #[test]
    fn inc_by_more_than_one() {
        let h = linear(100, 10);

        let mut b = HistogramBatch::from_histogram(&h);

        b.measure(0, 3);
        assert_bucket_eq!(b, 0, 3);
        assert_bucket_eq!(b, 1, 0);

        b.measure(50, 5);
        assert_bucket_eq!(b, 0, 8);
        assert_bucket_eq!(b, 1, 0);

        b.measure(100, 2);
        assert_bucket_eq!(b, 0, 8);
        assert_bucket_eq!(b, 1, 2);
        assert_bucket_eq!(b, 2, 0);

        b.measure(101, 19);
        assert_bucket_eq!(b, 0, 8);
        assert_bucket_eq!(b, 1, 21);
        assert_bucket_eq!(b, 2, 0);

        for bucket in h.buckets.iter() {
            assert_eq!(bucket.load(Relaxed), 0);
        }

        b.submit(&h);

        for i in 0..h.buckets.len() {
            assert_eq!(h.buckets[i].load(Relaxed), b.buckets[i]);
        }

        b.submit(&h);

        for i in 0..h.buckets.len() {
            assert_eq!(h.buckets[i].load(Relaxed), b.buckets[i]);
        }
    }
}
