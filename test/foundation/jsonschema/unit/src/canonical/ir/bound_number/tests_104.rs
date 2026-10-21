#[cfg(test)]
mod tests {
    use super::*;

    // A total order needs exactly one of a < b, a > b, a == b for every pair.
    #[test]
    fn ordering_is_antisymmetric() {
        let bounds: Vec<BoundNumber> = ["2", "2.5", "-3", "0", "1e100"]
            .iter()
            .flat_map(|text| {
                let number: Number = text.parse().expect("number");
                [
                    BoundNumber::new(&number, true),
                    BoundNumber::new(&number, false),
                ]
            })
            .collect();
        for left in &bounds {
            for right in &bounds {
                assert_eq!(
                    left.cmp(right),
                    right.cmp(left).reverse(),
                    "{left:?} vs {right:?}"
                );
            }
        }
    }
}
