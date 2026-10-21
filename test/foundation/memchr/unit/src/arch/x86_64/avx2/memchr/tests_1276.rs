#[cfg(test)]
mod tests {
    use super::*;

    define_memchr_quickcheck!(super);

    #[test]
    fn forward_one() {
        crate::tests::memchr::Runner::new(1).forward_iter(
            |haystack, needles| {
                Some(One::new(needles[0])?.iter(haystack).collect())
            },
        )
    }

    #[test]
    fn reverse_one() {
        crate::tests::memchr::Runner::new(1).reverse_iter(
            |haystack, needles| {
                Some(One::new(needles[0])?.iter(haystack).rev().collect())
            },
        )
    }

    #[test]
    fn count_one() {
        crate::tests::memchr::Runner::new(1).count_iter(|haystack, needles| {
            Some(One::new(needles[0])?.iter(haystack).count())
        })
    }

    #[test]
    fn forward_two() {
        crate::tests::memchr::Runner::new(2).forward_iter(
            |haystack, needles| {
                let n1 = needles.get(0).copied()?;
                let n2 = needles.get(1).copied()?;
                Some(Two::new(n1, n2)?.iter(haystack).collect())
            },
        )
    }

    #[test]
    fn reverse_two() {
        crate::tests::memchr::Runner::new(2).reverse_iter(
            |haystack, needles| {
                let n1 = needles.get(0).copied()?;
                let n2 = needles.get(1).copied()?;
                Some(Two::new(n1, n2)?.iter(haystack).rev().collect())
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
                Some(Three::new(n1, n2, n3)?.iter(haystack).collect())
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
                Some(Three::new(n1, n2, n3)?.iter(haystack).rev().collect())
            },
        )
    }
}
