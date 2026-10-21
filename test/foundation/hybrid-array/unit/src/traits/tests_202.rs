#[cfg(test)]
mod tests {
    use super::{AsArrayMut, AsArrayRef, SliceExt};
    use crate::{
        Array,
        sizes::{U2, U3},
    };

    type A = Array<u8, U2>;

    #[test]
    fn core_as_array_ref() {
        assert_eq!([1, 2, 3].as_array_ref(), &Array([1, 2, 3]));
    }

    #[test]
    fn core_as_array_mut() {
        assert_eq!([1, 2, 3].as_array_mut(), &Array([1, 2, 3]));
    }

    #[test]
    fn hybrid_as_array_ref() {
        assert_eq!(A::from([1, 2]).as_array_ref(), &Array([1, 2]));
    }

    #[test]
    fn hybrid_as_array_mut() {
        assert_eq!(A::from([1, 2]).as_array_mut(), &Array([1, 2]));
    }

    #[test]
    fn slice_as_hybrid_array() {
        assert_eq!([1, 2].as_hybrid_array::<U3>(), None);
        assert_eq!([1, 2, 3].as_hybrid_array::<U3>(), Some(&Array([1, 2, 3])));
        assert_eq!([1, 2, 3, 4].as_hybrid_array::<U3>(), None);
    }

    #[test]
    fn slice_as_mut_hybrid_array() {
        assert_eq!([1, 2].as_mut_hybrid_array::<U3>(), None);
        assert_eq!(
            [1, 2, 3].as_mut_hybrid_array::<U3>(),
            Some(&mut Array([1, 2, 3]))
        );
        assert_eq!([1, 2, 3, 4].as_mut_hybrid_array::<U3>(), None);
    }

    #[test]
    fn slice_as_hybrid_chunks() {
        let (slice_empty, rem_empty): (&[A], &[u8]) = [].as_hybrid_chunks::<U2>();
        assert!(slice_empty.is_empty());
        assert!(rem_empty.is_empty());

        let (slice_one, rem_one) = [1].as_hybrid_chunks::<U2>();
        assert!(slice_one.is_empty());
        assert_eq!(rem_one, &[1]);

        let (slice_aligned, rem_aligned) = [1u8, 2].as_hybrid_chunks::<U2>();
        assert_eq!(slice_aligned, &[Array([1u8, 2])]);
        assert_eq!(rem_aligned, b"");

        let (slice_unaligned, rem_unaligned) = [1u8, 2, 3].as_hybrid_chunks::<U2>();
        assert_eq!(slice_unaligned, &[Array([1u8, 2])]);
        assert_eq!(rem_unaligned, &[3]);
    }

    #[test]
    fn slice_as_hybrid_chunks_mut() {
        let (slice_empty, rem_empty): (&mut [A], &mut [u8]) = [].as_hybrid_chunks_mut::<U2>();
        assert!(slice_empty.is_empty());
        assert!(rem_empty.is_empty());

        let mut arr1 = [1];
        let (slice_one, rem_one) = arr1.as_hybrid_chunks_mut::<U2>();
        assert!(slice_one.is_empty());
        assert_eq!(rem_one, &[1]);

        let mut arr2 = [1u8, 2];
        let (slice_aligned, rem_aligned) = arr2.as_hybrid_chunks_mut::<U2>();
        assert_eq!(slice_aligned, &mut [Array([1u8, 2])]);
        assert_eq!(rem_aligned, b"");

        let mut arr3 = [1u8, 2, 3];
        let (slice_unaligned, rem_unaligned) = arr3.as_hybrid_chunks_mut::<U2>();
        assert_eq!(slice_unaligned, &mut [Array([1u8, 2])]);
        assert_eq!(rem_unaligned, &mut [3]);
    }
}
