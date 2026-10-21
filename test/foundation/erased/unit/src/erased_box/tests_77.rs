#[cfg(test)]
mod tests {
    use crate::ErasedBox;

    #[test]
    fn basic_test() {
        let erased = ErasedBox::new(Box::new(5usize));
        let r2 = unsafe { erased.into_inner::<usize>() };
        assert_eq!(*r2, 5);
    }

    #[test]
    fn ref_test() {
        let mut erased = ErasedBox::new(Box::new(5usize));
        assert_eq!(*unsafe { erased.get_ref::<usize>() }, 5);
        *unsafe { erased.get_mut::<usize>() } = 42;
        assert_eq!(*unsafe { erased.get_ref::<usize>() }, 42);

        // Drop `erased`
        unsafe { erased.into_inner::<usize>() };
    }
}
