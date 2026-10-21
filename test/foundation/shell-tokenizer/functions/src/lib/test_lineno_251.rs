#[test]
fn test_lineno() {
    let mut sh = Shlex::new("\nfoo\nbar");
    while let Some(word) = sh.next() {
        if word == "bar" {
            assert_eq!(sh.line_no, 3);
        }
    }
}
