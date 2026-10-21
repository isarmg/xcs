#[cfg(test)]
mod tests {
    use super::*;
    use crate::pointer::cast::Project as _;

    fn test_size_eq<Src, Dst: SizeEq<Src>>(mut src: Src) {
        let _: *mut Dst =
            <Dst as SizeEq<Src>>::CastFrom::project(crate::pointer::PtrInner::from_mut(&mut src));
    }

    #[test]
    fn test_transmute_coverage() {
        // SizeEq<T> for MaybeUninit<T>
        test_size_eq::<u8, MaybeUninit<u8>>(0u8);

        // SizeEq<MaybeUninit<T>> for T
        test_size_eq::<MaybeUninit<u8>, u8>(MaybeUninit::<u8>::new(0));

        // Transitive: MaybeUninit<T> -> Wrapping<T>
        // T => MaybeUninit<T> => T => Wrapping<T>
        test_size_eq::<u8, Wrapping<u8>>(0u8);

        // T => Wrapping<T> => T => MaybeUninit<T>
        test_size_eq::<Wrapping<u8>, MaybeUninit<u8>>(Wrapping(0u8));

        // T: ?Sized => Cell<T> => T => UnsafeCell<T>
        test_size_eq::<Cell<u8>, UnsafeCell<u8>>(Cell::new(0u8));

        // T: ?Sized => UnsafeCell<T> => T => Cell<T>
        test_size_eq::<UnsafeCell<u8>, Cell<u8>>(UnsafeCell::new(0u8));
    }
}
