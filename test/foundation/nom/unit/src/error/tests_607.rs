#[cfg(test)]
#[cfg(feature = "alloc")]
mod tests {
  use super::*;
  use crate::character::complete::char;

  #[test]
  fn convert_error_panic() {
    let input = "";

    let _result: IResult<_, _, VerboseError<&str>> = char('x')(input);
  }
}
