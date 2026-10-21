#[test]
    fn test_debug_impl() {
        // Ensure that Debug applies format options to the inner value.
        let val = U16::<LE>::new(10);
        assert_eq!(format!("{:?}", val), "U16(10)");
        assert_eq!(format!("{:03?}", val), "U16(010)");
        assert_eq!(format!("{:x?}", val), "U16(a)");
    }
