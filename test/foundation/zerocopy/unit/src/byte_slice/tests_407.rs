#[cfg(test)]
mod tests {
    use core::cell::RefCell;

    use super::*;

    #[test]
    fn test_ref_split_at_unchecked() {
        let cell = RefCell::new([1, 2, 3, 4]);
        let borrow = cell.borrow();
        let slice_ref: cell::Ref<'_, [u8]> = cell::Ref::map(borrow, |a| &a[..]);
        // SAFETY: 2 is within bounds of [1, 2, 3, 4]
        let (l, r) = unsafe { slice_ref.split_at_unchecked(2) };
        assert_eq!(*l, [1, 2]);
        assert_eq!(*r, [3, 4]);
    }

    #[test]
    fn test_ref_mut_split_at_unchecked() {
        let cell = RefCell::new([1, 2, 3, 4]);
        let borrow_mut = cell.borrow_mut();
        let slice_ref_mut: cell::RefMut<'_, [u8]> = cell::RefMut::map(borrow_mut, |a| &mut a[..]);
        // SAFETY: 2 is within bounds of [1, 2, 3, 4]
        let (mut l, mut r) = unsafe { slice_ref_mut.split_at_unchecked(2) };
        assert_eq!(*l, [1, 2]);
        assert_eq!(*r, [3, 4]);
        l[0] = 5;
        r[0] = 6;
        drop((l, r));
        assert_eq!(*cell.borrow(), [5, 2, 6, 4]);
    }
}
