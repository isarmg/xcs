#[test]
    fn test_byteorder_traits_coverage() {
        let val_be = U16::<BigEndian>::from_bytes([0, 1]);
        let val_le = U16::<LittleEndian>::from_bytes([1, 0]);

        assert_eq!(val_be.get(), 1);
        assert_eq!(val_le.get(), 1);

        // Debug
        assert_eq!(format!("{:?}", val_be), "U16(1)");
        assert_eq!(format!("{:?}", val_le), "U16(1)");

        // PartialOrd, Ord with same type
        assert!(val_be >= val_be);
        assert!(val_be <= val_be);
        assert_eq!(val_be.cmp(&val_be), core::cmp::Ordering::Equal);

        let low = U16::<LE>::new(255);
        let high = U16::<LE>::new(256);
        assert!(low < high);
        assert!(low < 256u16);
        assert_eq!(low.cmp(&high), core::cmp::Ordering::Less);
        assert!(I16::<LE>::new(-1) < I16::<LE>::new(0));

        // PartialOrd with native
        assert!(val_be == 1u16);
        assert!(val_be >= 1u16);

        // Default
        let default_be: U16<BigEndian> = Default::default();
        assert_eq!(default_be.get(), 0);

        // I16
        let val_be_i16 = I16::<BigEndian>::from_bytes([0, 1]);
        assert_eq!(val_be_i16.get(), 1);
        assert_eq!(format!("{:?}", val_be_i16), "I16(1)");
        assert_eq!(val_be_i16.cmp(&val_be_i16), core::cmp::Ordering::Equal);
    }
