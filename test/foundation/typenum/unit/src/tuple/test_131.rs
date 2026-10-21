#[cfg(test)]
mod test {
    use crate::{Len, U0, U1, U11, U12, U2};

    #[test]
    fn tuple_index() {
        assert_eq!((0,)[U0::new()], 0);
        assert_eq!((1, 2)[U1::new()], 2);
        assert_eq!((1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12)[U11::new()], 12);
    }

    #[test]
    fn tuple_index_mut() {
        let mut tuple = (0, 1, 2);

        tuple[U0::new()] = 5;

        assert_eq!(tuple, (5, 1, 2));
    }

    #[test]
    fn tuple_len() {
        assert_eq!(().len(), U0::new());
        assert_eq!((1,).len(), U1::new());
        assert_eq!((1, 2).len(), U2::new());
        assert_eq!((1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12).len(), U12::new());
    }
}
