#[test]
fn test_checked_binomial() {
    // With the first row: [1, 0, 0, ...] and the first column full of 1s, we check
    // row by row the recurrence relation of binomials (which is an equivalent definition).
    // For n >= 1 and k >= 1 we have:
    //   binomial(n, k) == binomial(n - 1, k - 1) + binomial(n - 1, k)
    const LIMIT: usize = 500;
    let mut row = vec![Some(0); LIMIT + 1];
    row[0] = Some(1);
    for n in 0..=LIMIT {
        for k in 0..=LIMIT {
            assert_eq!(row[k], checked_binomial(n, k));
        }
        row = std::iter::once(Some(1))
            .chain((1..=LIMIT).map(|k| row[k - 1]?.checked_add(row[k]?)))
            .collect();
    }
}
