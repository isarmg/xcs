#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn choice_serialization() {
        let expected = vec![
            ColorChoice::Auto,
            ColorChoice::AlwaysAnsi,
            ColorChoice::Always,
            ColorChoice::Never,
        ];
        let values: Vec<_> = expected
            .iter()
            .cloned()
            .map(AtomicChoice::from_choice)
            .collect();
        let actual: Vec<_> = values
            .iter()
            .cloned()
            .filter_map(AtomicChoice::to_choice)
            .collect();
        assert_eq!(expected, actual);
    }
}
