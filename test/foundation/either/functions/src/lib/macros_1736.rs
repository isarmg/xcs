#[test]
fn macros() {
    use std::string::String;

    fn a() -> Either<u32, u32> {
        let x: u32 = try_left!(Right(1337u32));
        Left(x * 2)
    }
    assert_eq!(a(), Right(1337));

    fn b() -> Either<String, &'static str> {
        Right(try_right!(Left("foo bar")))
    }
    assert_eq!(b(), Left(String::from("foo bar")));
}
