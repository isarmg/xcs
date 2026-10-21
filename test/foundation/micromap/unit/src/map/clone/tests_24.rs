#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_can_be_cloned() {
        let mut m: Map<u8, u8, 16> = Map::new();
        m.insert(0, 42);
        assert_eq!(42, *m.clone().get(&0).unwrap());
    }

    #[test]
    fn empty_map_can_be_cloned() {
        let m: Map<u8, u8, 0> = Map::new();
        assert!(m.clone().is_empty());
    }
}
