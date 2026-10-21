#[cfg(test)]
mod test_map {
    use super::*;

    #[test]
    fn test_prev_pow2() {
        // Skip 0, not defined for that input.
        let mut pow2: usize = 1;
        while (pow2 << 1) > 0 {
            let next_pow2 = pow2 << 1;
            assert_eq!(pow2, prev_pow2(pow2));
            // Need to skip 2, because it's also a power of 2, so it doesn't
            // return the previous power of 2.
            if next_pow2 > 2 {
                assert_eq!(pow2, prev_pow2(pow2 + 1));
                assert_eq!(pow2, prev_pow2(next_pow2 - 1));
            }
            pow2 = next_pow2;
        }
    }

    #[test]
    fn test_minimum_capacity_for_small_types() {
        #[track_caller]
        fn test_t<T>() {
            let raw_table: RawTable<T> = RawTable::with_capacity(1);
            let actual_buckets = raw_table.num_buckets();
            let min_buckets = Group::WIDTH / core::mem::size_of::<T>();
            assert!(
                actual_buckets >= min_buckets,
                "expected at least {min_buckets} buckets, got {actual_buckets} buckets"
            );
        }

        test_t::<u8>();

        // This is only "small" for some platforms, like x86_64 with SSE2, but
        // there's no harm in running it on other platforms.
        test_t::<u16>();
    }

    fn rehash_in_place<T>(table: &mut RawTable<T>, hasher: impl Fn(&T) -> u64) {
        unsafe {
            table.table.rehash_in_place(
                &|table, index| hasher(table.bucket::<T>(index).as_ref()),
                mem::size_of::<T>(),
                if mem::needs_drop::<T>() {
                    Some(|ptr| ptr::drop_in_place(ptr.cast::<T>()))
                } else {
                    None
                },
            );
        }
    }

    #[test]
    fn rehash() {
        let mut table = RawTable::new();
        let hasher = |i: &u64| *i;
        for i in 0..100 {
            table.insert(i, i, hasher);
        }

        for i in 0..100 {
            unsafe {
                assert_eq!(table.find(i, |x| *x == i).map(|b| b.read()), Some(i));
            }
            assert!(table.find(i + 100, |x| *x == i + 100).is_none());
        }

        rehash_in_place(&mut table, hasher);

        for i in 0..100 {
            unsafe {
                assert_eq!(table.find(i, |x| *x == i).map(|b| b.read()), Some(i));
            }
            assert!(table.find(i + 100, |x| *x == i + 100).is_none());
        }
    }

    /// CHECKING THAT WE ARE NOT TRYING TO READ THE MEMORY OF
    /// AN UNINITIALIZED TABLE DURING THE DROP
    #[test]
    fn test_drop_uninitialized() {
        use stdalloc::vec::Vec;

        let table = unsafe {
            // SAFETY: The `buckets` is power of two and we're not
            // trying to actually use the returned RawTable.
            RawTable::<(u64, Vec<i32>)>::new_uninitialized(Global, 8, Fallibility::Infallible)
                .unwrap()
        };
        drop(table);
    }

    /// CHECKING THAT WE DON'T TRY TO DROP DATA IF THE `ITEMS`
    /// ARE ZERO, EVEN IF WE HAVE `FULL` CONTROL BYTES.
    #[test]
    fn test_drop_zero_items() {
        use stdalloc::vec::Vec;
        unsafe {
            // SAFETY: The `buckets` is power of two and we're not
            // trying to actually use the returned RawTable.
            let mut table =
                RawTable::<(u64, Vec<i32>)>::new_uninitialized(Global, 8, Fallibility::Infallible)
                    .unwrap();

            // WE SIMULATE, AS IT WERE, A FULL TABLE.

            // SAFETY: We checked that the table is allocated and therefore the table already has
            // `self.bucket_mask + 1 + Group::WIDTH` number of control bytes (see TableLayout::calculate_layout_for)
            // so writing `table.table.num_ctrl_bytes() == bucket_mask + 1 + Group::WIDTH` bytes is safe.
            table.table.ctrl_slice().fill_empty();

            // SAFETY: table.capacity() is guaranteed to be smaller than table.num_buckets()
            table.table.ctrl(0).write_bytes(0, table.capacity());

            // Fix up the trailing control bytes. See the comments in set_ctrl
            // for the handling of tables smaller than the group width.
            if table.num_buckets() < Group::WIDTH {
                // SAFETY: We have `self.bucket_mask + 1 + Group::WIDTH` number of control bytes,
                // so copying `self.num_buckets() == self.bucket_mask + 1` bytes with offset equal to
                // `Group::WIDTH` is safe
                table
                    .table
                    .ctrl(0)
                    .copy_to(table.table.ctrl(Group::WIDTH), table.table.num_buckets());
            } else {
                // SAFETY: We have `self.bucket_mask + 1 + Group::WIDTH` number of
                // control bytes,so copying `Group::WIDTH` bytes with offset equal
                // to `self.num_buckets() == self.bucket_mask + 1` is safe
                table
                    .table
                    .ctrl(0)
                    .copy_to(table.table.ctrl(table.table.num_buckets()), Group::WIDTH);
            }
            drop(table);
        }
    }

    /// CHECKING THAT WE DON'T TRY TO DROP DATA IF THE `ITEMS`
    /// ARE ZERO, EVEN IF WE HAVE `FULL` CONTROL BYTES.
    #[test]
    #[cfg(panic = "unwind")]
    fn test_catch_panic_clone_from() {
        use super::{AllocError, Allocator, Global};
        use core::sync::atomic::{AtomicI8, Ordering};
        use std::thread;
        use stdalloc::sync::Arc;
        use stdalloc::vec::Vec;

        struct MyAllocInner {
            drop_count: Arc<AtomicI8>,
        }

        #[derive(Clone)]
        struct MyAlloc {
            _inner: Arc<MyAllocInner>,
        }

        impl Drop for MyAllocInner {
            fn drop(&mut self) {
                println!("MyAlloc freed.");
                self.drop_count.fetch_sub(1, Ordering::SeqCst);
            }
        }

        unsafe impl Allocator for MyAlloc {
            fn allocate(&self, layout: Layout) -> std::result::Result<NonNull<[u8]>, AllocError> {
                let g = Global;
                g.allocate(layout)
            }

            unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
                unsafe {
                    let g = Global;
                    g.deallocate(ptr, layout);
                }
            }
        }

        const DISARMED: bool = false;
        const ARMED: bool = true;

        struct CheckedCloneDrop {
            panic_in_clone: bool,
            dropped: bool,
            need_drop: Vec<u64>,
        }

        impl Clone for CheckedCloneDrop {
            fn clone(&self) -> Self {
                if self.panic_in_clone {
                    panic!("panic in clone")
                }
                Self {
                    panic_in_clone: self.panic_in_clone,
                    dropped: self.dropped,
                    need_drop: self.need_drop.clone(),
                }
            }
        }

        impl Drop for CheckedCloneDrop {
            fn drop(&mut self) {
                if self.dropped {
                    panic!("double drop");
                }
                self.dropped = true;
            }
        }

        let dropped: Arc<AtomicI8> = Arc::new(AtomicI8::new(2));

        let mut table = RawTable::new_in(MyAlloc {
            _inner: Arc::new(MyAllocInner {
                drop_count: dropped.clone(),
            }),
        });

        for (idx, panic_in_clone) in core::iter::repeat_n(DISARMED, 7).enumerate() {
            let idx = idx as u64;
            table.insert(
                idx,
                (
                    idx,
                    CheckedCloneDrop {
                        panic_in_clone,
                        dropped: false,
                        need_drop: vec![idx],
                    },
                ),
                |(k, _)| *k,
            );
        }

        assert_eq!(table.len(), 7);

        thread::scope(|s| {
            let result = s.spawn(|| {
                let armed_flags = [
                    DISARMED, DISARMED, ARMED, DISARMED, DISARMED, DISARMED, DISARMED,
                ];
                let mut scope_table = RawTable::new_in(MyAlloc {
                    _inner: Arc::new(MyAllocInner {
                        drop_count: dropped.clone(),
                    }),
                });
                for (idx, &panic_in_clone) in armed_flags.iter().enumerate() {
                    let idx = idx as u64;
                    scope_table.insert(
                        idx,
                        (
                            idx,
                            CheckedCloneDrop {
                                panic_in_clone,
                                dropped: false,
                                need_drop: vec![idx + 100],
                            },
                        ),
                        |(k, _)| *k,
                    );
                }
                table.clone_from(&scope_table);
            });
            assert!(result.join().is_err());
        });

        // Let's check that all iterators work fine and do not return elements
        // (especially `RawIterRange`, which does not depend on the number of
        // elements in the table, but looks directly at the control bytes)
        //
        // SAFETY: We know for sure that `RawTable` will outlive
        // the returned `RawIter / RawIterRange` iterator.
        assert_eq!(table.len(), 0);
        assert_eq!(unsafe { table.iter().count() }, 0);
        assert_eq!(unsafe { table.iter().iter.count() }, 0);

        for idx in 0..table.num_buckets() {
            let idx = idx as u64;
            assert!(
                table.find(idx, |(k, _)| *k == idx).is_none(),
                "Index: {idx}"
            );
        }

        // All allocator clones should already be dropped.
        assert_eq!(dropped.load(Ordering::SeqCst), 1);
    }
}
