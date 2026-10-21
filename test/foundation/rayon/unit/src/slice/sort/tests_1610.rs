#[cfg(test)]
mod tests {
    use super::heapsort;
    use super::split_for_merge;
    use rand::distr::Uniform;
    use rand::{rng, Rng};

    #[test]
    fn test_heapsort() {
        let rng = &mut rng();

        for len in (0..25).chain(500..501) {
            for &modulus in &[5, 10, 100] {
                let dist = Uniform::new(0, modulus).unwrap();
                for _ in 0..100 {
                    let v: Vec<i32> = rng.sample_iter(&dist).take(len).collect();

                    // Test heapsort using `<` operator.
                    let mut tmp = v.clone();
                    heapsort(&mut tmp, |a, b| a < b);
                    assert!(tmp.windows(2).all(|w| w[0] <= w[1]));

                    // Test heapsort using `>` operator.
                    let mut tmp = v.clone();
                    heapsort(&mut tmp, |a, b| a > b);
                    assert!(tmp.windows(2).all(|w| w[0] >= w[1]));
                }
            }
        }

        // Sort using a completely random comparison function.
        // This will reorder the elements *somehow*, but won't panic.
        let mut v: Vec<_> = (0..100).collect();
        heapsort(&mut v, |_, _| rand::rng().random());
        heapsort(&mut v, |a, b| a < b);

        for (i, &entry) in v.iter().enumerate() {
            assert_eq!(entry, i);
        }
    }

    #[test]
    fn test_split_for_merge() {
        fn check(left: &[u32], right: &[u32]) {
            let (l, r) = split_for_merge(left, right, &|&a, &b| a < b);
            assert!(left[..l]
                .iter()
                .all(|&x| right[r..].iter().all(|&y| x <= y)));
            assert!(right[..r].iter().all(|&x| left[l..].iter().all(|&y| x < y)));
        }

        check(&[1, 2, 2, 2, 2, 3], &[1, 2, 2, 2, 2, 3]);
        check(&[1, 2, 2, 2, 2, 3], &[]);
        check(&[], &[1, 2, 2, 2, 2, 3]);

        let rng = &mut rng();

        for _ in 0..100 {
            let limit: u32 = rng.random_range(1..21);
            let left_len: usize = rng.random_range(0..20);
            let right_len: usize = rng.random_range(0..20);

            let mut left = rng
                .sample_iter(&Uniform::new(0, limit).unwrap())
                .take(left_len)
                .collect::<Vec<_>>();
            let mut right = rng
                .sample_iter(&Uniform::new(0, limit).unwrap())
                .take(right_len)
                .collect::<Vec<_>>();

            left.sort();
            right.sort();
            check(&left, &right);
        }
    }
}
