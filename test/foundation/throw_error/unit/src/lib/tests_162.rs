#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as StdError;

    #[derive(Debug)]
    struct MyError;

    impl Display for MyError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "MyError")
        }
    }

    impl StdError for MyError {}

    #[test]
    fn test_from() {
        let e = MyError;
        let _le = Error::from(e);

        let e = "some error".to_string();
        let _le = Error::from(e);

        let e = anyhow::anyhow!("anyhow error");
        let _le = Error::from(e);
    }
}
