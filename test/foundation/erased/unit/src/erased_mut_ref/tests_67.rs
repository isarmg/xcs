#[cfg(test)]
mod tests {
    use crate::ErasedMut;

    #[test]
    fn basic_test() {
        let r1 = &mut 5usize;
        let mut erased = ErasedMut::new(r1);
        let r2 = unsafe { erased.get::<usize>() };
        *r2 = 42;
        assert_eq!(*unsafe { erased.get_ref::<usize>() }, 42);
        assert_eq!(*r1, 42);
    }
}
