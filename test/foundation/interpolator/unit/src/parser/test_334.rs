#[cfg(test)]
#[cfg(feature = "iter")]
mod test {
    use super::*;
    #[test]
    fn collect_parenthesized() {
        use super::collect_parenthesized;
        let mut idx = 0;
        let mut format = "()";
        assert_eq!(collect_parenthesized(&mut format, &mut idx, 0).unwrap(), "");
        assert_eq!(idx, 2);
        assert_eq!(format, "");
        let mut idx = 0;
        let mut format = "#(a)))# hello";
        assert_eq!(
            collect_parenthesized(&mut format, &mut idx, 0).unwrap(),
            "a))"
        );
        assert_eq!(idx, 7);
        assert_eq!(format, " hello");
        let mut idx = 0;
        let mut format = "###((a:)#>))##))###)### hello";
        assert_eq!(
            collect_parenthesized(&mut format, &mut idx, 0).unwrap(),
            "(a:)#>))##)"
        );
        assert_eq!(idx, 19);
        assert_eq!(format, ")### hello");

        assert_eq!(
            collect_parenthesized(&mut "###()##", &mut 0, 10).unwrap_err(),
            Error::Parse(ParseError::FormatSpecUnclosed(10))
        );
        assert_eq!(
            collect_parenthesized(&mut "###", &mut 0, 10).unwrap_err(),
            Error::Parse(ParseError::FormatSpecUnclosed(10))
        );
        assert_eq!(
            collect_parenthesized(&mut "###a", &mut 0, 10).unwrap_err(),
            Error::Parse(ParseError::Expected("(", 3))
        );
    }
}
