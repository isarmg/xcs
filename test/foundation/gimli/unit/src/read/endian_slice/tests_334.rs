#[cfg(test)]
mod tests {
    use super::*;
    use crate::endianity::NativeEndian;

    #[test]
    fn test_endian_slice_split_at() {
        let endian = NativeEndian;
        let slice = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 0];
        let eb = EndianSlice::new(slice, endian);
        assert_eq!(
            eb.split_at(3),
            (
                EndianSlice::new(&slice[..3], endian),
                EndianSlice::new(&slice[3..], endian)
            )
        );
    }

    #[test]
    #[should_panic]
    fn test_endian_slice_split_at_out_of_bounds() {
        let slice = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 0];
        let eb = EndianSlice::new(slice, NativeEndian);
        eb.split_at(30);
    }
}
