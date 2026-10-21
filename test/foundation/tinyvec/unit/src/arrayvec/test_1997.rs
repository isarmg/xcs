#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn retain_mut_empty_vec() {
    let mut av: ArrayVec<[i32; 4]> = ArrayVec::new();
    av.retain_mut(|&mut x| x % 2 == 0);
    assert_eq!(av.len(), 0);
  }

  #[test]
  fn retain_mut_all_elements() {
    let mut av: ArrayVec<[i32; 4]> = array_vec!([i32; 4] => 2, 4, 6, 8);
    av.retain_mut(|&mut x| x % 2 == 0);
    assert_eq!(av.len(), 4);
    assert_eq!(av.as_slice(), &[2, 4, 6, 8]);
  }

  #[test]
  fn retain_mut_some_elements() {
    let mut av: ArrayVec<[i32; 4]> = array_vec!([i32; 4] => 1, 2, 3, 4);
    av.retain_mut(|&mut x| x % 2 == 0);
    assert_eq!(av.len(), 2);
    assert_eq!(av.as_slice(), &[2, 4]);
  }

  #[test]
  fn retain_mut_no_elements() {
    let mut av: ArrayVec<[i32; 4]> = array_vec!([i32; 4] => 1, 3, 5, 7);
    av.retain_mut(|&mut x| x % 2 == 0);
    assert_eq!(av.len(), 0);
  }

  #[test]
  fn retain_mut_zero_capacity() {
    let mut av: ArrayVec<[i32; 0]> = ArrayVec::new();
    av.retain_mut(|&mut x| x % 2 == 0);
    assert_eq!(av.len(), 0);
  }

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

    let av: ArrayVec<[S; 2]> = ArrayVec::from(ar);
    let mut buf_av = alloc::string::String::new();
    write!(&mut buf_av, "{av:#?}").unwrap();

    assert_eq!(buf_av, buf_ar)
  }

  #[cfg(feature = "experimental_write_impl")]
  #[test]
  #[allow(non_snake_case)]
  fn test_ArrayVec_experimental_write_impl() {
    let mut buffer: ArrayVec<[u8; 10]> = ArrayVec::default();
    assert_eq!(buffer.len(), 0);
    use core::fmt::Write;
    write!(buffer, "abc").unwrap();
    assert_eq!(buffer.as_slice(), b"abc");
  }
}
