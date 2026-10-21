#[cfg(test)]
mod tests {
    use super::*;

    // I found this in the course of building out the benchmark suite for
    // rebar.
    #[test]
    fn regression_suffix_literal_count() {
        let _ = env_logger::try_init();

        let re = Regex::new(r"[a-zA-Z]+ing").unwrap();
        assert_eq!(1, re.find_iter("tingling").count());
    }
}
