#[cfg(test)]
mod tests {
    use super::*;
    use crate::LittleEndian;

    #[test]
    fn test_endian_vec() {
        let mut w = EndianVec::new(LittleEndian);
        assert_eq!(w.endian(), LittleEndian);
        assert_eq!(w.len(), 0);

        w.write(&[1, 2]).unwrap();
        assert_eq!(w.slice(), &[1, 2]);
        assert_eq!(w.len(), 2);

        w.write(&[3, 4, 5]).unwrap();
        assert_eq!(w.slice(), &[1, 2, 3, 4, 5]);
        assert_eq!(w.len(), 5);

        w.write_at(0, &[6, 7]).unwrap();
        assert_eq!(w.slice(), &[6, 7, 3, 4, 5]);
        assert_eq!(w.len(), 5);

        w.write_at(3, &[8, 9]).unwrap();
        assert_eq!(w.slice(), &[6, 7, 3, 8, 9]);
        assert_eq!(w.len(), 5);

        assert_eq!(w.write_at(4, &[6, 7]), Err(Error::LengthOutOfBounds));
        assert_eq!(w.write_at(5, &[6, 7]), Err(Error::LengthOutOfBounds));
        assert_eq!(w.write_at(6, &[6, 7]), Err(Error::OffsetOutOfBounds));

        assert_eq!(w.into_vec(), vec![6, 7, 3, 8, 9]);
    }
}
