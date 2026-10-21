#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let err = parse.parse_next(&mut "").unwrap_err();
        assert!(matches!(err, ErrMode::Backtrack(CustomError::MyError)));
    }
}
