#[cfg(test)]
mod test {
    use super::*;
    use std::ops::Add;

    #[test]
    fn check_fold_chunks_with() {
        let words = "bishbashbosh!"
            .chars()
            .collect::<Vec<_>>()
            .into_par_iter()
            .fold_chunks_with(4, String::new(), |mut s, c| {
                s.push(c);
                s
            })
            .collect::<Vec<_>>();

        assert_eq!(words, vec!["bish", "bash", "bosh", "!"]);
    }

    // 'closure' value for tests below
    fn sum<T, U>(x: T, y: U) -> T
    where
        T: Add<U, Output = T>,
    {
        x + y
    }

    #[test]
    #[should_panic(expected = "chunk_size must not be zero")]
    fn check_fold_chunks_zero_size() {
        let _: Vec<i32> = vec![1, 2, 3]
            .into_par_iter()
            .fold_chunks_with(0, 0, sum)
            .collect();
    }

    #[test]
    fn check_fold_chunks_even_size() {
        assert_eq!(
            vec![1 + 2 + 3, 4 + 5 + 6, 7 + 8 + 9],
            (1..10)
                .into_par_iter()
                .fold_chunks_with(3, 0, sum)
                .collect::<Vec<i32>>()
        );
    }

    #[test]
    fn check_fold_chunks_with_empty() {
        let v: Vec<i32> = vec![];
        let expected: Vec<i32> = vec![];
        assert_eq!(
            expected,
            v.into_par_iter()
                .fold_chunks_with(2, 0, sum)
                .collect::<Vec<i32>>()
        );
    }

    #[test]
    fn check_fold_chunks_len() {
        assert_eq!(4, (0..8).into_par_iter().fold_chunks_with(2, 0, sum).len());
        assert_eq!(3, (0..9).into_par_iter().fold_chunks_with(3, 0, sum).len());
        assert_eq!(3, (0..8).into_par_iter().fold_chunks_with(3, 0, sum).len());
        assert_eq!(1, [1].par_iter().fold_chunks_with(3, 0, sum).len());
        assert_eq!(0, (0..0).into_par_iter().fold_chunks_with(3, 0, sum).len());
    }

    #[test]
    fn check_fold_chunks_uneven() {
        let cases: Vec<(Vec<u32>, usize, Vec<u32>)> = vec![
            ((0..5).collect(), 3, vec![1 + 2, 3 + 4]),
            (vec![1], 5, vec![1]),
            ((0..4).collect(), 3, vec![1 + 2, 3]),
        ];

        for (i, (v, n, expected)) in cases.into_iter().enumerate() {
            let mut res: Vec<u32> = vec![];
            v.par_iter()
                .fold_chunks_with(n, 0, sum)
                .collect_into_vec(&mut res);
            assert_eq!(expected, res, "Case {i} failed");

            res.truncate(0);
            v.into_par_iter()
                .fold_chunks_with(n, 0, sum)
                .rev()
                .collect_into_vec(&mut res);
            assert_eq!(
                expected.into_iter().rev().collect::<Vec<u32>>(),
                res,
                "Case {i} reversed failed"
            );
        }
    }
}
