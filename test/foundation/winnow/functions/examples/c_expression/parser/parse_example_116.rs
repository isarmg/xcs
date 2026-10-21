#[test]
fn parse_example() {
    // Check out the [`crate::parser::test`] in this example for more samples.
    let result = pratt_parser
        .parse("a*a + b - c")
        .map(|r| format!("{}", r))
        .unwrap();

    let expect = "\
SUB
  ADD
    MUL
      NAME a
      NAME a
    NAME b
  NAME c

(- (+ (* a a) b) c)";

    assert_eq!(result, expect);
}
