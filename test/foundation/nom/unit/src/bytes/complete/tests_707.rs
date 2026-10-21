#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn complete_take_while_m_n_utf8_all_matching() {
    let result: IResult<&str, &str> =
      super::take_while_m_n(1, 4, |c: char| c.is_alphabetic())("øn");
    assert_eq!(result, Ok(("", "øn")));
  }

  #[test]
  fn complete_take_while_m_n_utf8_all_matching_substring() {
    let result: IResult<&str, &str> =
      super::take_while_m_n(1, 1, |c: char| c.is_alphabetic())("øn");
    assert_eq!(result, Ok(("n", "ø")));
  }

  // issue #1336 "escaped hangs if normal parser accepts empty"
  fn escaped_string(input: &str) -> IResult<&str, &str> {
    use crate::character::complete::{alpha0, one_of};
    escaped(alpha0, '\\', one_of("n"))(input)
  }

  // issue #1336 "escaped hangs if normal parser accepts empty"
  #[test]
  fn escaped_hang() {
    escaped_string("7").unwrap();
    escaped_string("a7").unwrap();
  }

  // issue ##1118 escaped does not work with empty string
  fn unquote<'a>(input: &'a str) -> IResult<&'a str, &'a str> {
    use crate::bytes::complete::*;
    use crate::character::complete::*;
    use crate::combinator::opt;
    use crate::sequence::delimited;

    delimited(
      char('"'),
      escaped(opt(none_of(r#"\""#)), '\\', one_of(r#"\"rnt"#)),
      char('"'),
    )(input)
  }

  #[test]
  fn escaped_hang_1118() {
    assert_eq!(unquote(r#""""#), Ok(("", "")));
  }
}
