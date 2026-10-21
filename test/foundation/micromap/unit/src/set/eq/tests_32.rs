#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_two_sets() {
        let mut s1: Set<i32, 1> = Set::new();
        s1.insert(1);
        let mut s2: Set<i32, 1> = Set::new();
        s2.insert(1);
        assert_eq!(s1, s2);
    }

    #[test]
    fn compares_sets_with_different_order() {
        let mut s1: Set<i32, 3> = Set::new();
        s1.insert(1);
        s1.insert(2);
        s1.insert(3);
        let mut s2: Set<i32, 3> = Set::new();
        s2.insert(3);
        s2.insert(2);
        s2.insert(1);
        assert_eq!(s1, s2);
    }

    #[test]
    fn compares_sets_with_different_lengths() {
        let mut s1: Set<i32, 3> = Set::new();
        s1.insert(1);
        s1.insert(2);
        let mut s2: Set<i32, 3> = Set::new();
        s2.insert(1);
        assert_ne!(s1, s2);
    }

    #[test]
    fn compares_sets_with_different_elements() {
        let mut s1: Set<i32, 1> = Set::new();
        s1.insert(1);
        let mut s2: Set<i32, 1> = Set::new();
        s2.insert(2);
        assert_ne!(s1, s2);
    }

    #[test]
    fn compares_sets_with_char_elements() {
        let mut s1: Set<char, 2> = Set::new();
        s1.insert('a');
        s1.insert('b');
        let mut s2: Set<char, 3> = Set::new();
        s2.insert('a');
        s2.insert('b');
        assert_eq!(s1, s2);
    }
}
