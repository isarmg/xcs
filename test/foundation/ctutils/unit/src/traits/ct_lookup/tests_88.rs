#[cfg(test)]
mod tests {
    mod array {
        use crate::CtLookup;

        const EXAMPLE: [u8; 3] = [1, 2, 3];

        #[test]
        fn ct_lookup_u32() {
            assert_eq!(EXAMPLE.ct_lookup(0u32).unwrap(), 1);
            assert_eq!(EXAMPLE.ct_lookup(1u32).unwrap(), 2);
            assert_eq!(EXAMPLE.ct_lookup(2u32).unwrap(), 3);
            assert!(EXAMPLE.ct_lookup(3u32).is_none().to_bool());
            assert!(EXAMPLE.ct_lookup(4u32).is_none().to_bool());
        }

        #[test]
        fn ct_lookup_usize() {
            assert_eq!(EXAMPLE.ct_lookup(0usize).unwrap(), 1);
            assert_eq!(EXAMPLE.ct_lookup(1usize).unwrap(), 2);
            assert_eq!(EXAMPLE.ct_lookup(2usize).unwrap(), 3);
            assert!(EXAMPLE.ct_lookup(3usize).is_none().to_bool());
            assert!(EXAMPLE.ct_lookup(4usize).is_none().to_bool());
        }
    }

    mod slice {
        use crate::CtLookup;

        const EXAMPLE: &[u8] = &[1, 2, 3];

        #[test]
        fn ct_lookup_u32() {
            assert_eq!(EXAMPLE.ct_lookup(0u32).unwrap(), 1);
            assert_eq!(EXAMPLE.ct_lookup(1u32).unwrap(), 2);
            assert_eq!(EXAMPLE.ct_lookup(2u32).unwrap(), 3);
            assert!(EXAMPLE.ct_lookup(3u32).is_none().to_bool());
            assert!(EXAMPLE.ct_lookup(4u32).is_none().to_bool());
        }

        #[test]
        fn ct_lookup_usize() {
            assert_eq!(EXAMPLE.ct_lookup(0usize).unwrap(), 1);
            assert_eq!(EXAMPLE.ct_lookup(1usize).unwrap(), 2);
            assert_eq!(EXAMPLE.ct_lookup(2usize).unwrap(), 3);
            assert!(EXAMPLE.ct_lookup(3usize).is_none().to_bool());
            assert!(EXAMPLE.ct_lookup(4usize).is_none().to_bool());
        }
    }
}
