#[test]
fn test_par_sort_stability() {
    for len in (2..25).chain(500..510).chain(50_000..50_010) {
        for _ in 0..10 {
            let mut counts = [0; 10];

            // Create a vector like [(6, 1), (5, 1), (6, 2), ...],
            // where the first item of each tuple is random, but
            // the second item represents which occurrence of that
            // number this element is, i.e. the second elements
            // will occur in sorted order.
            let mut rng = rng();
            let mut v: Vec<_> = (0..len)
                .map(|_| {
                    let n: usize = rng.random_range(0..10);
                    counts[n] += 1;
                    (n, counts[n])
                })
                .collect();

            // Only sort on the first element, so an unstable sort
            // may mix up the counts.
            v.par_sort_by(|&(a, _), &(b, _)| a.cmp(&b));

            // This comparison includes the count (the second item
            // of the tuple), so elements with equal first items
            // will need to be ordered with increasing
            // counts... i.e. exactly asserting that this sort is
            // stable.
            assert!(v.windows(2).all(|w| w[0] <= w[1]));
        }
    }
}
