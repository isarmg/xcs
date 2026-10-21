#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_two_maps() {
        let mut m1: Map<String, i32, 5> = Map::new();
        m1.insert("first".to_string(), 42);
        let mut m2: Map<String, i32, 5> = Map::new();
        m2.insert("first".to_string(), 42);
        assert!(m1.eq(&m2));
    }

    #[test]
    fn compares_two_diff_cap_maps() {
        let mut m1: Map<char, i32, 3> = Map::from([('a', 97), ('b', 98), ('c', 99)]);
        let mut m2: Map<char, i32, 4> = Map::from([('c', 99), ('c', 99), ('c', 99), ('b', 98)]);
        m2.insert('a', 97);
        assert!(m1.eq(&m2));
        m1.remove(&'c');
        assert!(m1.ne(&m2));
    }
}
