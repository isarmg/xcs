#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn scheme_eq_to_str() {
        assert_eq!(&scheme("http"), "http");
        assert_eq!(&scheme("https"), "https");
        assert_eq!(&scheme("ftp"), "ftp");
        assert_eq!(&scheme("my+funky+scheme"), "my+funky+scheme");
    }

    #[test]
    fn invalid_scheme_is_error() {
        Scheme::try_from("my_funky_scheme").expect_err("Unexpectedly valid Scheme");

        // Invalid UTF-8
        Scheme::try_from([0xC0].as_ref()).expect_err("Unexpectedly valid Scheme");
    }

    fn scheme(s: &str) -> Scheme {
        s.parse().expect(&format!("Invalid scheme: {}", s))
    }
}
