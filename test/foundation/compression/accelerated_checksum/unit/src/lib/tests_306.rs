#[cfg(test)]
mod tests {
  #[test]
  fn test_from_checksum() {
    let buf = b"rust is pretty cool man";
    let sum = 0xdeadbeaf;

    let mut simd = super::Adler32::from_checksum(sum);
    let mut adler = crate::checksum::Adler32::from_checksum(sum);

    simd.write(buf);
    adler.write_slice(buf);

    let simd = simd.finish();
    let scalar = adler.checksum();

    assert_eq!(simd, scalar);
  }
}
