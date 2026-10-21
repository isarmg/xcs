#[cfg(all(test, unix, any(feature = "nightly", feature = "allocator-api2")))]
mod test_map_with_mmap_allocations {
    use super::HashMap;
    use crate::raw::prev_pow2;
    use core::alloc::Layout;
    use core::ptr::{NonNull, null_mut};

    #[cfg(feature = "nightly")]
    use core::alloc::{AllocError, Allocator};

    #[cfg(all(feature = "allocator-api2", not(feature = "nightly")))]
    use allocator_api2::alloc::{AllocError, Allocator};

    /// This is not a production quality allocator, just good enough for
    /// some basic tests.
    #[derive(Clone, Copy, Debug)]
    struct MmapAllocator {
        /// Guarantee this is a power of 2.
        page_size: usize,
    }

    impl MmapAllocator {
        fn new() -> Result<Self, AllocError> {
            let result = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
            if result < 1 {
                return Err(AllocError);
            }

            let page_size = result as usize;
            if page_size.is_power_of_two() {
                Ok(Self { page_size })
            } else {
                Err(AllocError)
            }
        }

        fn fit_to_page_size(&self, n: usize) -> Result<usize, AllocError> {
            // If n=0, give a single page (wasteful, I know).
            let n = if n == 0 { self.page_size } else { n };

            match n & (self.page_size - 1) {
                0 => Ok(n),
                rem => n.checked_add(self.page_size - rem).ok_or(AllocError),
            }
        }
    }

    unsafe impl Allocator for MmapAllocator {
        fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
            if layout.align() > self.page_size {
                return Err(AllocError);
            }

            let null = null_mut();
            let len = self.fit_to_page_size(layout.size())? as libc::size_t;
            let prot = libc::PROT_READ | libc::PROT_WRITE;
            let flags = libc::MAP_PRIVATE | libc::MAP_ANON;
            let addr = unsafe { libc::mmap(null, len, prot, flags, -1, 0) };

            // mmap returns MAP_FAILED on failure, not Null.
            if addr == libc::MAP_FAILED {
                return Err(AllocError);
            }

            if let Some(data) = NonNull::new(addr.cast()) {
                // SAFETY: this is NonNull::slice_from_raw_parts.
                Ok(unsafe {
                    NonNull::new_unchecked(core::ptr::slice_from_raw_parts_mut(data.as_ptr(), len))
                })
            } else {
                // This branch shouldn't be taken in practice, but since we
                // cannot return null as a valid pointer in our type system,
                // we attempt to handle it.
                _ = unsafe { libc::munmap(addr, len) };
                Err(AllocError)
            }
        }

        unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
            unsafe {
                // If they allocated it with this layout, it must round correctly.
                let size = self.fit_to_page_size(layout.size()).unwrap();
                let _result = libc::munmap(ptr.as_ptr().cast(), size);
                debug_assert_eq!(0, _result);
            }
        }
    }

    #[test]
    fn test_tiny_allocation_gets_rounded_to_page_size() {
        let alloc = MmapAllocator::new().unwrap();
        let mut map: HashMap<usize, (), _, _> = HashMap::with_capacity_in(1, alloc);

        // Size of an element plus its control byte.
        let rough_bucket_size = core::mem::size_of::<(usize, ())>() + 1;

        // Accounting for some misc. padding that's likely in the allocation
        // due to rounding to group width, etc.
        let overhead = 3 * core::mem::size_of::<usize>();
        let num_buckets = (alloc.page_size - overhead) / rough_bucket_size;
        // Buckets are always powers of 2.
        let min_elems = prev_pow2(num_buckets);
        // Real load-factor is 7/8, but this is a lower estimation, so 1/2.
        let min_capacity = min_elems >> 1;
        let capacity = map.capacity();
        assert!(
            capacity >= min_capacity,
            "failed: {capacity} >= {min_capacity}"
        );

        // Fill it up.
        for i in 0..capacity {
            map.insert(i, ());
        }
        // Capacity should not have changed and it should be full.
        assert_eq!(capacity, map.len());
        assert_eq!(capacity, map.capacity());

        // Alright, make it grow.
        map.insert(capacity, ());
        assert!(
            capacity < map.capacity(),
            "failed: {capacity} < {}",
            map.capacity()
        );
    }
}
