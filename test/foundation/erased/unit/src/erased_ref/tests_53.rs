#[cfg(test)]
mod tests {
    use crate::Erased;

    #[test]
    fn basic_test() {
        let r1 = &5usize;
        let erased = Erased::new(r1);
        let r2 = unsafe { erased.get::<usize>() };
        assert_eq!(r1, r2);
    }

    #[test]
    fn lifetimed_test() {
        let r1 = &5usize;
        let r2 = &r1;
        let erased = Erased::new(r2);
        let r3 = unsafe { erased.get::<&usize>() };
        assert_eq!(r2, r3);
    }

    #[test]
    fn heterogeneous_test() {
        let mut vec: Vec<Erased> = Vec::new();
        vec.push((&5u64).into());
        vec.push((&"Hello World").into());

        assert_eq!(unsafe { *vec[0].get::<u64>() }, 5);
        assert_eq!(unsafe { *vec[1].get::<&'static str>() }, "Hello World");
    }
}
