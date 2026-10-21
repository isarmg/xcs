#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_take_0() {
    let input = [].as_ref();
    let count = 0usize;
    assert_eq!(count, 0usize);
    let offset = 0usize;

    let result: crate::IResult<(&[u8], usize), usize> = take(count)((input, offset));

    assert_eq!(result, Ok(((input, offset), 0)));
  }

  #[test]
  fn test_tag_ok() {
    let input = [0b00011111].as_ref();
    let offset = 0usize;
    let bits_to_take = 4usize;
    let value_to_tag = 0b0001;

    let result: crate::IResult<(&[u8], usize), usize> =
      tag(value_to_tag, bits_to_take)((input, offset));

    assert_eq!(result, Ok(((input, bits_to_take), value_to_tag)));
  }

  #[test]
  fn test_tag_err() {
    let input = [0b00011111].as_ref();
    let offset = 0usize;
    let bits_to_take = 4usize;
    let value_to_tag = 0b1111;

    let result: crate::IResult<(&[u8], usize), usize> =
      tag(value_to_tag, bits_to_take)((input, offset));

    assert_eq!(
      result,
      Err(crate::Err::Error(crate::error::Error {
        input: (input, offset),
        code: ErrorKind::TagBits
      }))
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

    assert_eq!(result, Err(crate::Err::Incomplete(Needed::new(1))));
  }
}
