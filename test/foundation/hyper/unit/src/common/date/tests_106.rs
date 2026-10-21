#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "nightly")]
    use test::Bencher;

    #[test]
    fn test_date_len() {
        assert_eq!(DATE_VALUE_LENGTH, "Sun, 06 Nov 1994 08:49:37 GMT".len());
    }

    #[cfg(feature = "nightly")]
    #[bench]
    fn bench_date_check(b: &mut Bencher) {
        let mut date = CachedDate::new();
        // cache the first update
        date.check();

        b.iter(|| {
            date.check();
        });
    }

    #[cfg(feature = "nightly")]
    #[bench]
    fn bench_date_render(b: &mut Bencher) {
        let mut date = CachedDate::new();
        let now = SystemTime::now();
        date.render(now);
        b.bytes = date.buffer().len() as u64;

        b.iter(|| {
            date.render(now);
            test::black_box(&date);
        });
    }
}
