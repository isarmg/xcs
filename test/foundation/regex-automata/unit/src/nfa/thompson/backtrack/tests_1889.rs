#[cfg(test)]
mod tests {
    use super::*;

    // This is a regression test for the maximum haystack length computation.
    // Previously, it assumed that the total capacity of the backtracker's
    // bitset would always be greater than the number of NFA states. But there
    // is of course no guarantee that this is true. This regression test
    // ensures that not only does `max_haystack_len` not panic, but that it
    // should return `0`.
    #[cfg(feature = "syntax")]
    #[test]
    fn max_haystack_len_overflow() {
        let re = BoundedBacktracker::builder()
            .configure(BoundedBacktracker::config().visited_capacity(10))
            .build(r"[0-9A-Za-z]{100}")
            .unwrap();
        assert_eq!(0, re.max_haystack_len());
    }
}
