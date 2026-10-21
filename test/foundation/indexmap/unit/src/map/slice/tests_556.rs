#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slice_index() {
        fn check(
            vec_slice: &[(i32, i32)],
            map_slice: &Slice<i32, i32>,
            sub_slice: &Slice<i32, i32>,
        ) {
            assert_eq!(map_slice as *const _, sub_slice as *const _);
            iterator_toolkit::assert_equal(
                vec_slice.iter().copied(),
                map_slice.iter().map(|(&k, &v)| (k, v)),
            );
            iterator_toolkit::assert_equal(vec_slice.iter().map(|(k, _)| k), map_slice.keys());
            iterator_toolkit::assert_equal(vec_slice.iter().map(|(_, v)| v), map_slice.values());
        }

        let vec: Vec<(i32, i32)> = (0..10).map(|i| (i, i * i)).collect();
        let map: IndexMap<i32, i32> = vec.iter().cloned().collect();
        let slice = map.as_slice();

        // RangeFull
        #[expect(clippy::redundant_slicing)]
        check(&vec[..], &map[..], &slice[..]);

        for i in 0usize..10 {
            // Index
            assert_eq!(vec[i].1, map[i]);
            assert_eq!(vec[i].1, slice[i]);
            assert_eq!(map[&(i as i32)], map[i]);
            assert_eq!(map[&(i as i32)], slice[i]);

            // RangeFrom
            check(&vec[i..], &map[i..], &slice[i..]);

            // RangeTo
            check(&vec[..i], &map[..i], &slice[..i]);

            // RangeToInclusive
            check(&vec[..=i], &map[..=i], &slice[..=i]);

            // (Bound<usize>, Bound<usize>)
            let bounds = (Bound::Excluded(i), Bound::Unbounded);
            check(&vec[i + 1..], &map[bounds], &slice[bounds]);

            for j in i..=10 {
                // Range
                check(&vec[i..j], &map[i..j], &slice[i..j]);
            }

            for j in i..10 {
                // RangeInclusive
                check(&vec[i..=j], &map[i..=j], &slice[i..=j]);
            }
        }
    }

    #[test]
    fn slice_index_mut() {
        fn check_mut(
            vec_slice: &[(i32, i32)],
            map_slice: &mut Slice<i32, i32>,
            sub_slice: &mut Slice<i32, i32>,
        ) {
            assert_eq!(map_slice, sub_slice);
            iterator_toolkit::assert_equal(
                vec_slice.iter().copied(),
                map_slice.iter_mut().map(|(&k, &mut v)| (k, v)),
            );
            iterator_toolkit::assert_equal(
                vec_slice.iter().map(|&(_, v)| v),
                map_slice.values_mut().map(|&mut v| v),
            );
        }

        let vec: Vec<(i32, i32)> = (0..10).map(|i| (i, i * i)).collect();
        let mut map: IndexMap<i32, i32> = vec.iter().cloned().collect();
        let mut map2 = map.clone();
        let slice = map2.as_mut_slice();

        // RangeFull
        check_mut(&vec[..], &mut map[..], &mut slice[..]);

        for i in 0usize..10 {
            // IndexMut
            assert_eq!(&mut map[i], &mut slice[i]);

            // RangeFrom
            check_mut(&vec[i..], &mut map[i..], &mut slice[i..]);

            // RangeTo
            check_mut(&vec[..i], &mut map[..i], &mut slice[..i]);

            // RangeToInclusive
            check_mut(&vec[..=i], &mut map[..=i], &mut slice[..=i]);

            // (Bound<usize>, Bound<usize>)
            let bounds = (Bound::Excluded(i), Bound::Unbounded);
            check_mut(&vec[i + 1..], &mut map[bounds], &mut slice[bounds]);

            for j in i..=10 {
                // Range
                check_mut(&vec[i..j], &mut map[i..j], &mut slice[i..j]);
            }

            for j in i..10 {
                // RangeInclusive
                check_mut(&vec[i..=j], &mut map[i..=j], &mut slice[i..=j]);
            }
        }
    }

    #[test]
    fn slice_new() {
        let slice: &Slice<i32, i32> = Slice::new();
        assert!(slice.is_empty());
        assert_eq!(slice.len(), 0);
    }

    #[test]
    fn slice_new_mut() {
        let slice: &mut Slice<i32, i32> = Slice::new_mut();
        assert!(slice.is_empty());
        assert_eq!(slice.len(), 0);
    }

    #[test]
    fn slice_get_index_mut() {
        let mut map: IndexMap<i32, i32> = (0..10).map(|i| (i, i * i)).collect();
        let slice: &mut Slice<i32, i32> = map.as_mut_slice();

        {
            let (key, value) = slice.get_index_mut(0).unwrap();
            assert_eq!(*key, 0);
            assert_eq!(*value, 0);

            *value = 11;
        }

        assert_eq!(slice[0], 11);

        {
            let result = slice.get_index_mut(11);
            assert!(result.is_none());
        }
    }

    #[test]
    fn slice_split_first() {
        let slice: &mut Slice<i32, i32> = Slice::new_mut();
        let result = slice.split_first();
        assert!(result.is_none());

        let mut map: IndexMap<i32, i32> = (0..10).map(|i| (i, i * i)).collect();
        let slice: &mut Slice<i32, i32> = map.as_mut_slice();

        {
            let (first, rest) = slice.split_first().unwrap();
            assert_eq!(first, (&0, &0));
            assert_eq!(rest.len(), 9);
        }
        assert_eq!(slice.len(), 10);
    }

    #[test]
    fn slice_split_first_mut() {
        let slice: &mut Slice<i32, i32> = Slice::new_mut();
        let result = slice.split_first_mut();
        assert!(result.is_none());

        let mut map: IndexMap<i32, i32> = (0..10).map(|i| (i, i * i)).collect();
        let slice: &mut Slice<i32, i32> = map.as_mut_slice();

        {
            let (first, rest) = slice.split_first_mut().unwrap();
            assert_eq!(first, (&0, &mut 0));
            assert_eq!(rest.len(), 9);

            *first.1 = 11;
        }
        assert_eq!(slice.len(), 10);
        assert_eq!(slice[0], 11);
    }

    #[test]
    fn slice_split_last() {
        let slice: &mut Slice<i32, i32> = Slice::new_mut();
        let result = slice.split_last();
        assert!(result.is_none());

        let mut map: IndexMap<i32, i32> = (0..10).map(|i| (i, i * i)).collect();
        let slice: &mut Slice<i32, i32> = map.as_mut_slice();

        {
            let (last, rest) = slice.split_last().unwrap();
            assert_eq!(last, (&9, &81));
            assert_eq!(rest.len(), 9);
        }
        assert_eq!(slice.len(), 10);
    }

    #[test]
    fn slice_split_last_mut() {
        let slice: &mut Slice<i32, i32> = Slice::new_mut();
        let result = slice.split_last_mut();
        assert!(result.is_none());

        let mut map: IndexMap<i32, i32> = (0..10).map(|i| (i, i * i)).collect();
        let slice: &mut Slice<i32, i32> = map.as_mut_slice();

        {
            let (last, rest) = slice.split_last_mut().unwrap();
            assert_eq!(last, (&9, &mut 81));
            assert_eq!(rest.len(), 9);

            *last.1 = 100;
        }

        assert_eq!(slice.len(), 10);
        assert_eq!(slice[slice.len() - 1], 100);
    }

    #[test]
    fn slice_get_range() {
        let mut map: IndexMap<i32, i32> = (0..10).map(|i| (i, i * i)).collect();
        let slice: &mut Slice<i32, i32> = map.as_mut_slice();
        let subslice = slice.get_range(3..6).unwrap();
        assert_eq!(subslice.len(), 3);
        assert_eq!(subslice, &[(3, 9), (4, 16), (5, 25)]);
    }
}
