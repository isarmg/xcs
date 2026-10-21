#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_fwd_done_range() {
        let smap = StartByteMap::new(&LookMatcher::default());
        let input = Input::new("").range(1..0);
        let config = Config::from_input_forward(&input);
        let start =
            config.get_look_behind().map_or(Start::Text, |b| smap.get(b));
        assert_eq!(Start::Text, start);
    }

    #[test]
    fn start_rev_done_range() {
        let smap = StartByteMap::new(&LookMatcher::default());
        let input = Input::new("").range(1..0);
        let config = Config::from_input_reverse(&input);
        let start =
            config.get_look_behind().map_or(Start::Text, |b| smap.get(b));
        assert_eq!(Start::Text, start);
    }

    #[test]
    fn start_fwd() {
        let f = |haystack, start, end| {
            let smap = StartByteMap::new(&LookMatcher::default());
            let input = Input::new(haystack).range(start..end);
            let config = Config::from_input_forward(&input);
            let start =
                config.get_look_behind().map_or(Start::Text, |b| smap.get(b));
            start
        };

        assert_eq!(Start::Text, f("", 0, 0));
        assert_eq!(Start::Text, f("abc", 0, 3));
        assert_eq!(Start::Text, f("\nabc", 0, 3));

        assert_eq!(Start::LineLF, f("\nabc", 1, 3));

        assert_eq!(Start::LineCR, f("\rabc", 1, 3));

        assert_eq!(Start::WordByte, f("abc", 1, 3));

        assert_eq!(Start::NonWordByte, f(" abc", 1, 3));
    }

    #[test]
    fn start_rev() {
        let f = |haystack, start, end| {
            let smap = StartByteMap::new(&LookMatcher::default());
            let input = Input::new(haystack).range(start..end);
            let config = Config::from_input_reverse(&input);
            let start =
                config.get_look_behind().map_or(Start::Text, |b| smap.get(b));
            start
        };

        assert_eq!(Start::Text, f("", 0, 0));
        assert_eq!(Start::Text, f("abc", 0, 3));
        assert_eq!(Start::Text, f("abc\n", 0, 4));

        assert_eq!(Start::LineLF, f("abc\nz", 0, 3));

        assert_eq!(Start::LineCR, f("abc\rz", 0, 3));

        assert_eq!(Start::WordByte, f("abc", 0, 2));

        assert_eq!(Start::NonWordByte, f("abc ", 0, 3));
    }
}
