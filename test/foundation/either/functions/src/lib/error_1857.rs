#[test]
fn error() {
    let invalid_utf8 = b"\xff";
    #[allow(invalid_from_utf8)]
    let res = if let Err(error) = ::std::str::from_utf8(invalid_utf8) {
        Err(Left(error))
    } else if let Err(error) = "x".parse::<i32>() {
        Err(Right(error))
    } else {
        Ok(())
    };
    assert!(res.is_err());
    #[allow(deprecated)]
    res.unwrap_err().description(); // make sure this can be called
}
