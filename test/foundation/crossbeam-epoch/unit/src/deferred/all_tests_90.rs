#[cfg(all(test, not(crossbeam_loom)))]
mod tests {
    use super::Deferred;
    use std::boxed::Box;
    use std::cell::Cell;
    use std::convert::identity;
    use std::string::ToString;
    use std::vec;

    #[test]
    fn on_stack() {
        let fired = &Cell::new(false);
        let a = [0usize; 1];

        let d = Deferred::new(move || {
            let _ = identity(a);
            fired.set(true);
        });

        assert!(!fired.get());
        d.call();
        assert!(fired.get());
    }

    #[test]
    fn on_heap() {
        let fired = &Cell::new(false);
        let a = [0usize; 10];

        let d = Deferred::new(move || {
            let _ = identity(a);
            fired.set(true);
        });

        assert!(!fired.get());
        d.call();
        assert!(fired.get());
    }

    #[test]
    fn string() {
        let a = "hello".to_string();
        let d = Deferred::new(move || assert_eq!(a, "hello"));
        d.call();
    }

    #[test]
    fn boxed_slice_i32() {
        let a: Box<[i32]> = vec![2, 3, 5, 7].into_boxed_slice();
        let d = Deferred::new(move || assert_eq!(*a, [2, 3, 5, 7]));
        d.call();
    }

    #[test]
    fn long_slice_usize() {
        let a: [usize; 5] = [2, 3, 5, 7, 11];
        let d = Deferred::new(move || assert_eq!(a, [2, 3, 5, 7, 11]));
        d.call();
    }
}
