#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward1_iter() {
        crate::tests::memchr::Runner::new(1).forward_iter(
            |haystack, needles| {
                Some(memchr_iter(needles[0], haystack).collect())
            },
        )
    }

    #[test]
    fn forward1_oneshot() {
        crate::tests::memchr::Runner::new(1).forward_oneshot(
            |haystack, needles| Some(memchr(needles[0], haystack)),
        )
    }

    #[test]
    fn reverse1_iter() {
        crate::tests::memchr::Runner::new(1).reverse_iter(
            |haystack, needles| {
                Some(memrchr_iter(needles[0], haystack).collect())
            },
        )
    }

    #[test]
    fn reverse1_oneshot() {
        crate::tests::memchr::Runner::new(1).reverse_oneshot(
            |haystack, needles| Some(memrchr(needles[0], haystack)),
        )
    }

    #[test]
    fn count1_iter() {
        crate::tests::memchr::Runner::new(1).count_iter(|haystack, needles| {
            Some(memchr_iter(needles[0], haystack).count())
        })
    }

    #[test]
    fn forward2_iter() {
        crate::tests::memchr::Runner::new(2).forward_iter(
            |haystack, needles| {
                let n1 = needles.get(0).copied()?;
                let n2 = needles.get(1).copied()?;
                Some(memchr2_iter(n1, n2, haystack).collect())
            },
        )
    }

    #[test]
    fn forward2_oneshot() {
        crate::tests::memchr::Runner::new(2).forward_oneshot(
            |haystack, needles| {
                let n1 = needles.get(0).copied()?;
                let n2 = needles.get(1).copied()?;
                Some(memchr2(n1, n2, haystack))
            },
        )
    }

    #[test]
    fn reverse2_iter() {
        crate::tests::memchr::Runner::new(2).reverse_iter(
            |haystack, needles| {
                let n1 = needles.get(0).copied()?;
                let n2 = needles.get(1).copied()?;
                Some(memrchr2_iter(n1, n2, haystack).collect())
            },
        )
    }

    #[test]
    fn reverse2_oneshot() {
        crate::tests::memchr::Runner::new(2).reverse_oneshot(
            |haystack, needles| {
                let n1 = needles.get(0).copied()?;
                let n2 = needles.get(1).copied()?;
                Some(memrchr2(n1, n2, haystack))
            },
        )
    }

    #[test]
    fn forward3_iter() {
        crate::tests::memchr::Runner::new(3).forward_iter(
            |haystack, needles| {
                let n1 = needles.get(0).copied()?;
                let n2 = needles.get(1).copied()?;
                let n3 = needles.get(2).copied()?;
                Some(memchr3_iter(n1, n2, n3, haystack).collect())
            },
        )
    }

    #[test]
    fn forward3_oneshot() {
        crate::tests::memchr::Runner::new(3).forward_oneshot(
            |haystack, needles| {
                let n1 = needles.get(0).copied()?;
                let n2 = needles.get(1).copied()?;
                let n3 = needles.get(2).copied()?;
                Some(memchr3(n1, n2, n3, haystack))
            },
        )
    }

    #[test]
    fn reverse3_iter() {
        crate::tests::memchr::Runner::new(3).reverse_iter(
            |haystack, needles| {
                let n1 = needles.get(0).copied()?;
                let n2 = needles.get(1).copied()?;
                let n3 = needles.get(2).copied()?;
                Some(memrchr3_iter(n1, n2, n3, haystack).collect())
            },
        )
    }

    #[test]
    fn reverse3_oneshot() {
        crate::tests::memchr::Runner::new(3).reverse_oneshot(
            |haystack, needles| {
                let n1 = needles.get(0).copied()?;
                let n2 = needles.get(1).copied()?;
                let n3 = needles.get(2).copied()?;
                Some(memrchr3(n1, n2, n3, haystack))
            },
        )
    }

    // Prior to memchr 2.6, the memchr iterators both implemented Send and
    // Sync. But in memchr 2.6, the iterator changed to use raw pointers
    // internally and I didn't add explicit Send/Sync impls. This ended up
    // regressing the API. This test ensures we don't do that again.
    //
    // See: https://github.com/BurntSushi/memchr/issues/133
    #[test]
    fn sync_regression() {
        use core::panic::{RefUnwindSafe, UnwindSafe};

        fn assert_send_sync<T: Send + Sync + UnwindSafe + RefUnwindSafe>() {}
        assert_send_sync::<Memchr>();
        assert_send_sync::<Memchr2>();
        assert_send_sync::<Memchr3>()
    }
}
