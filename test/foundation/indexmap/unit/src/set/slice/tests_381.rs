#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slice_index() {
        fn check(vec_slice: &[i32], set_slice: &Slice<i32>, sub_slice: &Slice<i32>) {
            assert_eq!(set_slice as *const _, sub_slice as *const _);
            iterator_toolkit::assert_equal(vec_slice, set_slice);
        }

        let vec: Vec<i32> = (0..10).map(|i| i * i).collect();
        let set: IndexSet<i32> = vec.iter().cloned().collect();
        let slice = set.as_slice();

        // RangeFull
        #[expect(clippy::redundant_slicing)]
        check(&vec[..], &set[..], &slice[..]);

        for i in 0usize..10 {
            // Index
            assert_eq!(vec[i], set[i]);
            assert_eq!(vec[i], slice[i]);

            // RangeFrom
            check(&vec[i..], &set[i..], &slice[i..]);

            // RangeTo
            check(&vec[..i], &set[..i], &slice[..i]);

            // RangeToInclusive
            check(&vec[..=i], &set[..=i], &slice[..=i]);

            // (Bound<usize>, Bound<usize>)
            let bounds = (Bound::Excluded(i), Bound::Unbounded);
            check(&vec[i + 1..], &set[bounds], &slice[bounds]);

            for j in i..=10 {
                // Range
                check(&vec[i..j], &set[i..j], &slice[i..j]);
            }

            for j in i..10 {
                // RangeInclusive
                check(&vec[i..=j], &set[i..=j], &slice[i..=j]);
            }
        }
    }
}
