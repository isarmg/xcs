#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_record_backwards_compat() {
        Span::current().record("some-key", "some text");
        Span::current().record("some-key", false);
    }
}
