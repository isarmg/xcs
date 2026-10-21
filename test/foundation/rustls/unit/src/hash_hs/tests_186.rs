#[cfg(test)]
#[macro_rules_attribute::apply(test_for_each_provider)]
mod tests {
    use super::provider::hash::SHA256;
    use super::*;
    use crate::crypto::hash::Hash;
    use crate::enums::ProtocolVersion;
    use crate::msgs::base::Payload;
    use crate::msgs::handshake::{HandshakeMessagePayload, HandshakePayload};

    #[test]
    fn hashes_correctly() {
        let mut hhb = HandshakeHashBuffer::new();
        hhb.add_raw(b"hello");
        assert_eq!(hhb.buffer.len(), 5);
        let mut hh = hhb.start_hash(&SHA256);
        assert!(hh.client_auth.is_none());
        hh.add_raw(b"world");
        let h = hh.current_hash();
        let h = h.as_ref();
        assert_eq!(h[0], 0x93);
        assert_eq!(h[1], 0x6a);
        assert_eq!(h[2], 0x18);
        assert_eq!(h[3], 0x5c);
    }

    #[test]
    fn hashes_message_types() {
        // handshake protocol encoding of 0x0e 00 00 00
        let server_hello_done_message = Message {
            version: ProtocolVersion::TLSv1_2,
            payload: MessagePayload::handshake(HandshakeMessagePayload(
                HandshakePayload::ServerHelloDone,
            )),
        };

        let app_data_ignored = Message {
            version: ProtocolVersion::TLSv1_3,
            payload: MessagePayload::ApplicationData(Payload::Borrowed(b"hello")),
        };

        let end_of_early_data_flight = Message {
            version: ProtocolVersion::TLSv1_3,
            payload: MessagePayload::HandshakeFlight(Payload::Borrowed(b"\x05\x00\x00\x00")),
        };

        // buffered mode
        let mut hhb = HandshakeHashBuffer::new();
        hhb.add_message(&server_hello_done_message);
        hhb.add_message(&app_data_ignored);
        hhb.add_message(&end_of_early_data_flight);
        assert_eq!(
            hhb.start_hash(&SHA256)
                .current_hash()
                .as_ref(),
            SHA256
                .hash(b"\x0e\x00\x00\x00\x05\x00\x00\x00")
                .as_ref()
        );

        // non-buffered mode
        let mut hh = HandshakeHashBuffer::new().start_hash(&SHA256);
        hh.add_message(&server_hello_done_message);
        hh.add_message(&app_data_ignored);
        hh.add_message(&end_of_early_data_flight);
        assert_eq!(
            hh.current_hash().as_ref(),
            SHA256
                .hash(b"\x0e\x00\x00\x00\x05\x00\x00\x00")
                .as_ref()
        );
    }

    #[cfg(feature = "tls12")]
    #[test]
    fn buffers_correctly() {
        let mut hhb = HandshakeHashBuffer::new();
        hhb.set_client_auth_enabled();
        hhb.add_raw(b"hello");
        assert_eq!(hhb.buffer.len(), 5);
        let mut hh = hhb.start_hash(&SHA256);
        assert_eq!(
            hh.client_auth
                .as_ref()
                .map(|buf| buf.len()),
            Some(5)
        );
        hh.add_raw(b"world");
        assert_eq!(
            hh.client_auth
                .as_ref()
                .map(|buf| buf.len()),
            Some(10)
        );
        let h = hh.current_hash();
        let h = h.as_ref();
        assert_eq!(h[0], 0x93);
        assert_eq!(h[1], 0x6a);
        assert_eq!(h[2], 0x18);
        assert_eq!(h[3], 0x5c);
        let buf = hh.take_handshake_buf();
        assert_eq!(Some(b"helloworld".to_vec()), buf);
    }

    #[test]
    fn abandon() {
        let mut hhb = HandshakeHashBuffer::new();
        hhb.set_client_auth_enabled();
        hhb.add_raw(b"hello");
        assert_eq!(hhb.buffer.len(), 5);
        let mut hh = hhb.start_hash(&SHA256);
        assert_eq!(
            hh.client_auth
                .as_ref()
                .map(|buf| buf.len()),
            Some(5)
        );
        hh.abandon_client_auth();
        assert_eq!(hh.client_auth, None);
        hh.add_raw(b"world");
        assert_eq!(hh.client_auth, None);
        let h = hh.current_hash();
        let h = h.as_ref();
        assert_eq!(h[0], 0x93);
        assert_eq!(h[1], 0x6a);
        assert_eq!(h[2], 0x18);
        assert_eq!(h[3], 0x5c);
    }

    #[test]
    fn clones_correctly() {
        let mut hhb = HandshakeHashBuffer::new();
        hhb.set_client_auth_enabled();
        hhb.add_raw(b"hello");
        assert_eq!(hhb.buffer.len(), 5);

        // Cloning the HHB should result in the same buffer and client auth state.
        let mut hhb_prime = hhb.clone();
        assert_eq!(hhb_prime.buffer, hhb.buffer);
        assert!(hhb_prime.client_auth_enabled);

        // Updating the HHB clone shouldn't affect the original.
        hhb_prime.add_raw(b"world");
        assert_eq!(hhb_prime.buffer.len(), 10);
        assert_ne!(hhb.buffer, hhb_prime.buffer);

        let hh = hhb.start_hash(&SHA256);
        let hh_hash = hh.current_hash();
        let hh_hash = hh_hash.as_ref();

        // Cloning the HH should result in the same current hash.
        let mut hh_prime = hh.clone();
        let hh_prime_hash = hh_prime.current_hash();
        let hh_prime_hash = hh_prime_hash.as_ref();
        assert_eq!(hh_hash, hh_prime_hash);

        // Updating the HH clone shouldn't affect the original.
        hh_prime.add_raw(b"goodbye");
        assert_eq!(hh.current_hash().as_ref(), hh_hash);
        assert_ne!(hh_prime.current_hash().as_ref(), hh_hash);
    }
}
