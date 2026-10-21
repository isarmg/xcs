#[test]
fn escaped() {
    let data = "\"tab:\\tafter tab, newline:\\nnew line, quote: \\\", emoji: \\u{1F602}, newline:\\nescaped whitespace: \\    abc\"";
    let result = parser::parse_string::<()>.parse(data);
    assert_eq!(
        result,
        Ok(String::from("tab:\tafter tab, newline:\nnew line, quote: \", emoji: 😂, newline:\nescaped whitespace: abc"))
    );
}
