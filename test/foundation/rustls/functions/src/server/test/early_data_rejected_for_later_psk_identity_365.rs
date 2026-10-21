#[test]
    fn early_data_rejected_for_later_psk_identity() {
        // RFC 9846 section 4.3.10: "In order to accept early data, the server MUST have
        // accepted a PSK cipher suite and selected the first key offered in the client's
        // "pre_shared_key" extension." The server may still resume with a later PSK.
        let (selected, early_data_accepted) = resume_with_early_data(1);
        assert_eq!(selected, Some(1));
        assert!(!early_data_accepted);
    }
