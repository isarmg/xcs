#[test]
    fn early_data_accepted_for_first_psk_identity() {
        let (selected, early_data_accepted) = resume_with_early_data(0);
        assert_eq!(selected, Some(0));
        assert!(early_data_accepted);
    }
