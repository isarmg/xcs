#[cfg(test)]
mod tests {
    use super::*;

    // Uses private type `ChunkFooter`.
    #[test]
    fn chunk_footer_is_five_words() {
        assert_eq!(mem::size_of::<ChunkFooter>(), mem::size_of::<usize>() * 6);
    }

    // Uses private `DEFAULT_CHUNK_SIZE_WITHOUT_FOOTER` and `FOOTER_SIZE`.
    #[test]
    fn allocated_bytes() {
        let mut b = Bump::with_capacity(1);

        assert_eq!(b.allocated_bytes(), DEFAULT_CHUNK_SIZE_WITHOUT_FOOTER);
        assert_eq!(
            b.allocated_bytes_including_metadata(),
            DEFAULT_CHUNK_SIZE_WITHOUT_FOOTER + FOOTER_SIZE
        );

        b.reset();

        assert_eq!(b.allocated_bytes(), DEFAULT_CHUNK_SIZE_WITHOUT_FOOTER);
        assert_eq!(
            b.allocated_bytes_including_metadata(),
            DEFAULT_CHUNK_SIZE_WITHOUT_FOOTER + FOOTER_SIZE
        );
    }

    // Uses private `alloc` module.
    #[test]
    fn test_realloc() {
        use crate::alloc::Alloc;

        unsafe {
            const CAPACITY: usize = DEFAULT_CHUNK_SIZE_WITHOUT_FOOTER;
            let mut b = Bump::<1>::with_min_align_and_capacity(CAPACITY);

            // `realloc` doesn't shrink allocations that aren't "worth it".
            let layout = Layout::from_size_align(100, 1).unwrap();
            let p = b.alloc_layout(layout);
            let q = (&b).realloc(p, layout, 51).unwrap();
            assert_eq!(p, q);
            b.reset();

            // `realloc` will shrink allocations that are "worth it".
            let layout = Layout::from_size_align(100, 1).unwrap();
            let p = b.alloc_layout(layout);
            let q = (&b).realloc(p, layout, 50).unwrap();
            assert!(p != q);
            b.reset();

            // `realloc` will reuse the last allocation when growing.
            let layout = Layout::from_size_align(10, 1).unwrap();
            let p = b.alloc_layout(layout);
            let q = (&b).realloc(p, layout, 11).unwrap();
            assert_eq!(q.as_ptr() as usize, p.as_ptr() as usize - 1);
            b.reset();

            // `realloc` will allocate a new chunk when growing the last
            // allocation, if need be.
            let layout = Layout::from_size_align(1, 1).unwrap();
            let p = b.alloc_layout(layout);
            let q = (&b).realloc(p, layout, CAPACITY + 1).unwrap();
            assert_ne!(q.as_ptr() as usize, p.as_ptr() as usize - CAPACITY);
            b.reset();

            // `realloc` will allocate and copy when reallocating anything that
            // wasn't the last allocation.
            let layout = Layout::from_size_align(1, 1).unwrap();
            let p = b.alloc_layout(layout);
            let _ = b.alloc_layout(layout);
            let q = (&b).realloc(p, layout, 2).unwrap();
            assert!(q.as_ptr() as usize != p.as_ptr() as usize - 1);
            b.reset();
        }
    }

    // Uses private `alloc` module.
    #[test]
    fn realloc_old_size_zero() {
        use crate::alloc::Alloc;

        let bump = Bump::new();

        let old_layout = Layout::from_size_align(0, 1).unwrap();
        let old_ptr = bump.alloc_layout(old_layout);
        let new_size = 64;
        let new_ptr = unsafe { (&bump).realloc(old_ptr, old_layout, new_size).unwrap() };
        let new_ptr = new_ptr.as_ptr().cast::<u8>();

        // Write to and read from the pointer. If it is invalid, then MIRI will
        // complain.
        unsafe {
            for i in 0..new_size {
                *new_ptr.add(i) = 0xAB;
            }
            for i in 0..new_size {
                assert_eq!(*new_ptr.add(i), 0xAB);
            }
        }
    }

    // Uses our private `alloc` module.
    #[test]
    fn invalid_read() {
        use alloc::Alloc;

        let mut b = &Bump::new();

        unsafe {
            let l1 = Layout::from_size_align(12000, 4).unwrap();
            let p1 = Alloc::alloc(&mut b, l1).unwrap();

            let l2 = Layout::from_size_align(1000, 4).unwrap();
            Alloc::alloc(&mut b, l2).unwrap();

            let p1 = b.realloc(p1, l1, 24000).unwrap();
            let l3 = Layout::from_size_align(24000, 4).unwrap();
            b.realloc(p1, l3, 48000).unwrap();
        }
    }
}
