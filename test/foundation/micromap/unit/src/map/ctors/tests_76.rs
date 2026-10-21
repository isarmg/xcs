#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn makes_default_map() {
        let m: Map<u8, u8, 8> = Map::default();
        assert_eq!(0, m.len());
    }

    #[test]
    fn makes_new_map() {
        let m: Map<u8, u8, 8> = Map::new();
        assert_eq!(0, m.len());
    }

    #[test]
    fn makes_map_by_with_capacity() {
        #[allow(deprecated)]
        let m: Map<u8, u8, 8> = Map::with_capacity(8);
        assert_eq!(0, m.len());
    }

    #[test]
    fn drops_correctly() {
        let m: Map<Vec<u8>, u8, 8> = Map::new();
        drop(m);
    }

    #[test]
    fn drops_keys() {
        use std::rc::Rc;
        let mut m: Map<Rc<()>, (), 8> = Map::new();
        let k = Rc::new(());
        m.insert(Rc::clone(&k), ());
        drop(m);
        assert_eq!(Rc::strong_count(&k), 1);
    }

    #[test]
    fn drops_values() {
        use std::rc::Rc;
        let mut m: Map<(), Rc<()>, 8> = Map::new();
        let v = Rc::new(());
        m.insert((), Rc::clone(&v));
        drop(m);
        assert_eq!(Rc::strong_count(&v), 1);
    }
}
