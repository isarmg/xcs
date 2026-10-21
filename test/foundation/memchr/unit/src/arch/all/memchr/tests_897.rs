#[cfg(test)]
mod tests {
    use super::*;

    define_memchr_quickcheck!(super, try_new);

    #[test]
    fn forward_one() {
        crate::tests::memchr::Runner::new(1).forward_iter(
            |haystack, needles| {
                Some(One::new(needles[0]).iter(haystack).collect())
            },
        )
    }

    #[test]
    fn reverse_one() {
        crate::tests::memchr::Runner::new(1).reverse_iter(
            |haystack, needles| {
                Some(One::new(needles[0]).iter(haystack).rev().collect())
            },
        )
    }

    #[test]
    fn count_one() {
        crate::tests::memchr::Runner::new(1).count_iter(|haystack, needles| {
            Some(One::new(needles[0]).iter(haystack).count())
        })
    }

    #[test]
    fn forward_two() {
        crate::tests::memchr::Runner::new(2).forward_iter(
            |haystack, needles| {
                let n1 = needles.get(0).copied()?;
                let n2 = needles.get(1).copied()?;
                Some(Two::new(n1, n2).iter(haystack).collect())
            },
        )
    }

    #[test]
    fn reverse_two() {
        crate::tests::memchr::Runner::new(2).reverse_iter(
            |haystack, needles| {
                let n1 = needles.get(0).copied()?;
                let n2 = needles.get(1).copied()?;
                Some(Two::new(n1, n2).iter(haystack).rev().collect())
            },
        )
    }

    #[test]
    fn forward_three() {
        crate::tests::memchr::Runner::new(3).forward_iter(
            |haystack, needles| {
                let n1 = needles.get(0).copied()?;
                let n2 = needles.get(1).copied()?;
                let n3 = needles.get(2).copied()?;
                Some(Three::new(n1, n2, n3).iter(haystack).collect())
            },
        )
    }

    #[test]
    fn reverse_three() {
        crate::tests::memchr::Runner::new(3).reverse_iter(
            |haystack, needles| {
                let n1 = needles.get(0).copied()?;
                let n2 = needles.get(1).copied()?;
                let n3 = needles.get(2).copied()?;
                Some(Three::new(n1, n2, n3).iter(haystack).rev().collect())
            },
        )
    }

    // This was found by quickcheck in the course of refactoring this crate
    // after memchr 2.5.0.
    #[test]
    fn regression_double_ended_iterator() {
        let finder = One::new(b'a');
        let haystack = "a";
        let mut it = finder.iter(haystack.as_bytes());
        assert_eq!(Some(0), it.next());
        assert_eq!(None, it.next_back());
    }

    // This regression test was caught by ripgrep's test suite on i686 when
    // upgrading to memchr 2.6. Namely, something about the \x0B bytes here
    // screws with the SWAR counting approach I was using. This regression test
    // prompted me to remove the SWAR counting approach and just replace it
    // with a byte-at-a-time loop.
    #[test]
    fn regression_count_new_lines() {
        let haystack = "01234567\x0b\n\x0b\n\x0b\n\x0b\nx";
        let count = One::new(b'\n').count(haystack.as_bytes());
        assert_eq!(4, count);
    }

    // A test[1] that failed on some big endian targets after a perf
    // improvement was merged[2].
    //
    // At first it seemed like the test suite somehow missed the regression,
    // but in actuality, CI was not running tests with `cross` but instead with
    // `cargo` specifically. This is because those steps were using `cargo`
    // instead of `${{ env.CARGO }}`. So adding this regression test doesn't
    // really help catch that class of failure, but we add it anyway for good
    // measure.
    //
    // [1]: https://github.com/BurntSushi/memchr/issues/152
    // [2]: https://github.com/BurntSushi/memchr/pull/151
    #[test]
    fn regression_big_endian1() {
        assert_eq!(One::new(b':').find(b"1:23"), Some(1));
    }

    // Interestingly, I couldn't get `regression_big_endian1` to fail for me
    // on the `powerpc64-unknown-linux-gnu` target. But I found another case
    // through quickcheck that does.
    #[test]
    fn regression_big_endian2() {
        let data = [0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(One::new(b'\x00').find(&data), Some(0));
    }
}
