#[test]
fn test_lineno() {
    let mut sh = Shlex::new(b"\nfoo\nbar");
    while let Some(word) = sh.next() {
        if word == b"bar" {
            assert_eq!(sh.line_no, 3);
        }
    }
}
