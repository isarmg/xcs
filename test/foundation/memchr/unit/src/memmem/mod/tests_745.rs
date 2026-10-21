#[cfg(test)]
mod tests {
    use super::*;

    define_substring_forward_quickcheck!(|h, n| Some(Finder::new(n).find(h)));
    define_substring_reverse_quickcheck!(|h, n| Some(
        FinderRev::new(n).rfind(h)
    ));

    #[test]
    fn forward() {
        crate::tests::substring::Runner::new()
            .fwd(|h, n| Some(Finder::new(n).find(h)))
            .run();
    }

    #[test]
    fn reverse() {
        crate::tests::substring::Runner::new()
            .rev(|h, n| Some(FinderRev::new(n).rfind(h)))
            .run();
    }
}
