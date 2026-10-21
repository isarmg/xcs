#[cfg(test)]
mod tests {
    use crate::{Array, sizes::U3};

    #[test]
    fn mut_array_ref_into_iterator() {
        let array_ref: &mut Array<u8, U3> = &mut Array([1, 2, 3]);
        let mut iter = array_ref.into_iter();

        for i in 1..=3 {
            assert_eq!(iter.next().copied(), Some(i));
        }

        assert_eq!(iter.next(), None);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn display_try_from_iterator_error() {
        use super::TryFromIteratorError;
        use alloc::string::ToString;
        let _ = TryFromIteratorError.to_string();
    }
}
