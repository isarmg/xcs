#[test]
fn indexed_example() {
    use crate::iter::plumbing::*;
    use crate::prelude::*;
    use crate::vec::IntoIter;

    struct MyIntoIter<T: Send> {
        inner: IntoIter<T>,
    }

    delegate_indexed_iterator! {
        MyIntoIter<T> => T,
        impl<T: Send>
    }

    let iter = MyIntoIter {
        inner: vec![1, 2, 3].into_par_iter(),
    };
    let mut vec = vec![];
    iter.collect_into_vec(&mut vec);
    assert_eq!(vec, &[1, 2, 3]);
}
