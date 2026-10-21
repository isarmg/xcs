#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    #[test]
    fn test_meta() {
        let arr = [1; 16];
        let dst = <[u8]>::ref_from_bytes(&arr[..]).unwrap();
        let ptr = PtrInner::from_ref(dst);
        assert_eq!(ptr.meta().get(), 16);

        // SAFETY: 8 is less than 16
        let ptr = unsafe { ptr.with_meta(8) };

        assert_eq!(ptr.meta().get(), 8);
    }

    #[test]
    fn test_split_at() {
        fn test_split_at<const OFFSET: usize, const BUFFER_SIZE: usize>() {
            #[derive(FromBytes, KnownLayout, SplitAt, Immutable)]
            #[repr(C)]
            struct SliceDst<const OFFSET: usize> {
                prefix: [u8; OFFSET],
                trailing: [u8],
            }

            let n: usize = BUFFER_SIZE - OFFSET;
            let arr = [1; BUFFER_SIZE];
            let dst = SliceDst::<OFFSET>::ref_from_bytes(&arr[..]).unwrap();
            let ptr = PtrInner::from_ref(dst);
            for i in 0..=n {
                assert_eq!(ptr.meta().get(), n);
                // SAFETY: `i` is in bounds by construction.
                let i = unsafe { MetadataOf::new_unchecked(i) };
                // SAFETY: `i` is in bounds by construction.
                let (l, r) = unsafe { ptr.split_at_unchecked(i) };
                // SAFETY: Points to a valid value by construction.
                #[allow(clippy::undocumented_unsafe_blocks, clippy::as_conversions)]
                // Clippy false positive
                let l_sum: usize = l
                    .trailing_slice()
                    .iter()
                    .map(
                        #[inline(always)]
                        |ptr| unsafe { core::ptr::read_unaligned(ptr.as_ptr()) } as usize,
                    )
                    .sum();
                // SAFETY: Points to a valid value by construction.
                #[allow(clippy::undocumented_unsafe_blocks, clippy::as_conversions)]
                // Clippy false positive
                let r_sum: usize = r
                    .iter()
                    .map(
                        #[inline(always)]
                        |ptr| unsafe { core::ptr::read_unaligned(ptr.as_ptr()) } as usize,
                    )
                    .sum();
                assert_eq!(l_sum, i.get());
                assert_eq!(r_sum, n - i.get());
                assert_eq!(l_sum + r_sum, n);
            }
        }

        test_split_at::<0, 16>();
        test_split_at::<1, 17>();
        test_split_at::<2, 18>();
    }

    #[test]
    fn test_trailing_slice() {
        fn test_trailing_slice<const OFFSET: usize, const BUFFER_SIZE: usize>() {
            #[derive(FromBytes, KnownLayout, SplitAt, Immutable)]
            #[repr(C)]
            struct SliceDst<const OFFSET: usize> {
                prefix: [u8; OFFSET],
                trailing: [u8],
            }

            let n: usize = BUFFER_SIZE - OFFSET;
            let arr = [1; BUFFER_SIZE];
            let dst = SliceDst::<OFFSET>::ref_from_bytes(&arr[..]).unwrap();
            let ptr = PtrInner::from_ref(dst);

            assert_eq!(ptr.meta().get(), n);
            let trailing = ptr.trailing_slice();
            assert_eq!(trailing.meta().get(), n);

            assert_eq!(
                // SAFETY: We assume this to be sound for the sake of this test,
                // which will fail, here, in miri, if the safety precondition of
                // `offset_of` is not satisfied.
                unsafe {
                    #[allow(clippy::as_conversions)]
                    let offset = (trailing.as_ptr() as *mut u8).offset_from(ptr.as_ptr() as *mut _);
                    offset
                },
                isize::try_from(OFFSET).unwrap(),
            );

            // SAFETY: Points to a valid value by construction.
            #[allow(clippy::undocumented_unsafe_blocks, clippy::as_conversions)]
            // Clippy false positive
            let trailing: usize = trailing
                .iter()
                .map(|ptr| unsafe { core::ptr::read_unaligned(ptr.as_ptr()) } as usize)
                .sum();

            assert_eq!(trailing, n);
        }

        test_trailing_slice::<0, 16>();
        test_trailing_slice::<1, 17>();
        test_trailing_slice::<2, 18>();
    }
    #[test]
    fn test_ptr_inner_clone() {
        let mut x = 0u8;
        let p = PtrInner::from_mut(&mut x);
        #[allow(clippy::clone_on_copy)]
        let p2 = p.clone();
        assert_eq!(p.as_non_null(), p2.as_non_null());
    }
}
