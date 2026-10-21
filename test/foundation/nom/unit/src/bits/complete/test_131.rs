#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_take_0() {
    let input = [0b00010010].as_ref();
    let count = 0usize;
    assert_eq!(count, 0usize);
    let offset = 0usize;

    let result: crate::IResult<(&[u8], usize), usize> = take(count)((input, offset));

    assert_eq!(result, Ok(((input, offset), 0)));
  }

  #[test]
  fn test_take_eof() {
    let input = [0b00010010].as_ref();

    let result: crate::IResult<(&[u8], usize), usize> = take(1usize)((input, 8));

    assert_eq!(
      result,
      Err(crate::Err::Error(crate::error::Error {
        input: (input, 8),
        code: ErrorKind::Eof
      }))
    )
  }

  #[test]
  fn test_take_span_over_multiple_bytes() {
    let input = [0b00010010, 0b00110100, 0b11111111, 0b11111111].as_ref();

    let result: crate::IResult<(&[u8], usize), usize> = take(24usize)((input, 4));

    assert_eq!(
      result,
      Ok((([0b11111111].as_ref(), 4), 0b1000110100111111111111))
    );
  }

  #[test]
  fn test_bool_0() {
    let input = [0b10000000].as_ref();

    let result: crate::IResult<(&[u8], usize), bool> = bool((input, 0));

    assert_eq!(result, Ok(((input, 1), true)));
  }

  #[test]
  fn test_bool_eof() {
    let input = [0b10000000].as_ref();

    let result: crate::IResult<(&[u8], usize), bool> = bool((input, 8));

    assert_eq!(
      result,
      Err(crate::Err::Error(crate::error::Error {
        input: (input, 8),
        code: ErrorKind::Eof
      }))
    );
  }
}
