#[cfg(test)]
mod tests {
    use super::CtFind;

    mod array {
        use super::*;
        use crate::{CtEq, CtGt};

        const ARRAY: [u8; 6] = [0, 0, 0, 1, 2, 3];

        #[test]
        fn ct_find() {
            // Find the first nonzero even number
            assert_eq!(
                ARRAY.ct_find(|n| n.ct_ne(&0) & (n & 1).ct_eq(&0)).unwrap(),
                2
            );

            // Predicate where nothing matches
            assert!(ARRAY.ct_find(|n| n.ct_gt(&3)).is_none().to_bool());
        }
    }

    mod slice {
        use super::*;
        use crate::{CtEq, CtGt};

        const SLICE: &[u8] = &[0, 0, 0, 1, 2, 3];

        #[test]
        fn ct_find() {
            // Find the first nonzero even number
            assert_eq!(
                SLICE.ct_find(|n| n.ct_ne(&0) & (n & 1).ct_eq(&0)).unwrap(),
                2
            );

            // Predicate where nothing matches
            assert!(SLICE.ct_find(|n| n.ct_gt(&3)).is_none().to_bool());
        }
    }
}
