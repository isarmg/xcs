#[test]
fn unindexed_example() {
    use crate::collections::btree_map::IntoIter;
    use crate::iter::plumbing::*;
    use crate::prelude::*;

    use std::collections::BTreeMap;

    struct MyIntoIter<T: Ord + Send, U: Send> {
        inner: IntoIter<T, U>,
    }

    delegate_iterator! {
        MyIntoIter<T, U> => (T, U),
        impl<T: Ord + Send, U: Send>
    }

    let map = BTreeMap::from([(1, 'a'), (2, 'b'), (3, 'c')]);
    let iter = MyIntoIter {
        inner: map.into_par_iter(),
    };
    let vec: Vec<_> = iter.map(|(k, _)| k).collect();
    assert_eq!(vec, &[1, 2, 3]);
}
