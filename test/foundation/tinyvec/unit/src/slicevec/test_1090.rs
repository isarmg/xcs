#[cfg(test)]
mod test {
  use super::*;

  #[cfg(feature = "alloc")]
  #[test]
  fn array_like_debug() {
    #[derive(Debug, Default, Copy, Clone)]
    #[allow(unused)]
    struct S {
      x: u8,
      y: u8,
    }

    use core::fmt::Write;

    let mut ar: [S; 2] = [S { x: 1, y: 2 }, S { x: 3, y: 4 }];
    let mut buf_ar = alloc::string::String::new();
    write!(&mut buf_ar, "{ar:#?}").unwrap();

    let av: SliceVec<S> = SliceVec::from(&mut ar);
    let mut buf_av = alloc::string::String::new();
    write!(&mut buf_av, "{av:#?}").unwrap();

    assert_eq!(buf_av, buf_ar)
  }
}
