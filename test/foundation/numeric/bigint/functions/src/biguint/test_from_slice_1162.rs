#[test]
    fn test_from_slice() {
        fn check(slice: &[u32], data: &[BigDigit]) {
            assert_eq!(*BigUint::from_slice(slice).data, *data);
        }
        check(&[1], &[1]);
        check(&[0, 0, 0], &[]);
        check(&[1, 2, 0, 0], &[1, 2]);
        check(&[0, 0, 1, 2], &[0, 0, 1, 2]);
        check(&[0, 0, 1, 2, 0, 0], &[0, 0, 1, 2]);
        check(&[-1i32 as u32], &[-1i32 as BigDigit]);
    }
