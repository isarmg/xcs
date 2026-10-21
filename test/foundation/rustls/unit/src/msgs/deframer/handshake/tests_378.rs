#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;
    use crate::msgs::deframer::DeframerIter;

    fn add_bytes(hs: &mut HandshakeDeframer, slice: &[u8], within: &[u8]) {
        let msg = InboundPlainMessage {
            typ: ContentType::Handshake,
            version: ProtocolVersion::TLSv1_3,
            payload: slice,
        };
        let locator = Locator::new(within);
        let discard = locator.locate(slice).end;
        hs.input_message(msg, &locator, discard);
    }

    #[test]
    fn coalesce() {
        let mut input = vec![0, 0, 0, 0x21, 0, 0, 0, 0, 0x01, 0xff, 0x00, 0x01];
        let mut hs = HandshakeDeframer::default();

        add_bytes(&mut hs, &input[3..4], &input);
        assert_eq!(hs.requires_coalesce(), None);
        add_bytes(&mut hs, &input[4..6], &input);
        assert_eq!(hs.requires_coalesce(), Some(0));
        add_bytes(&mut hs, &input[8..10], &input);
        assert_eq!(hs.requires_coalesce(), Some(0));

        std::println!("before: {hs:?}");
        hs.coalesce(&mut input).unwrap();
        std::println!("after:  {hs:?}");

        let (msg, discard) = hs.iter(&input).next().unwrap();
        std::println!("msg {msg:?} discard {discard:?}");
        assert_eq!(msg.typ, ContentType::Handshake);
        assert_eq!(msg.version, ProtocolVersion::TLSv1_3);
        assert_eq!(msg.payload, &[0x21, 0x00, 0x00, 0x01, 0xff]);

        input.drain(..discard);
        assert_eq!(input, &[0, 1]);
    }

    #[test]
    fn append() {
        let mut input = vec![0, 0, 0, 0x21, 0, 0, 5, 0, 0, 1, 2, 3, 4, 5, 0];
        let mut hs = HandshakeDeframer::default();

        add_bytes(&mut hs, &input[3..7], &input);
        add_bytes(&mut hs, &input[9..14], &input);
        assert_eq!(hs.spans.len(), 2);

        hs.coalesce(&mut input).unwrap();
        assert_eq!(hs.spans.len(), 1);

        let (msg, discard) = std::dbg!(hs.iter(&input).next().unwrap());
        assert_eq!(msg.typ, ContentType::Handshake);
        assert_eq!(msg.version, ProtocolVersion::TLSv1_3);
        assert_eq!(msg.payload, &[0x21, 0x00, 0x00, 0x05, 1, 2, 3, 4, 5]);

        input.drain(..discard);
        assert_eq!(input, &[0]);
    }

    #[test]
    fn coalesce_rejects_excess_size_message() {
        const X: u8 = 0xff;
        let mut input = vec![0x21, 0x01, 0x00, X, 0x00, 0xab, X];
        let mut hs = HandshakeDeframer::default();

        // split header over multiple messages, which motivates doing
        // this check in `coalesce()`
        add_bytes(&mut hs, &input[0..3], &input);
        add_bytes(&mut hs, &input[4..6], &input);

        assert_eq!(
            hs.coalesce(&mut input),
            Err(InvalidMessage::HandshakePayloadTooLarge)
        );
    }

    #[test]
    fn iter_only_returns_full_messages() {
        let input = [0, 0, 0, 0x21, 0, 0, 1, 0xab, 0x21, 0, 0, 1];

        let mut hs = HandshakeDeframer::default();

        add_bytes(&mut hs, &input[3..8], &input);
        add_bytes(&mut hs, &input[8..12], &input);

        let mut iter = hs.iter(&input);
        let (msg, discard) = iter.next().unwrap();
        assert!(iter.next().is_none());

        assert_eq!(msg.typ, ContentType::Handshake);
        assert_eq!(msg.version, ProtocolVersion::TLSv1_3);
        assert_eq!(msg.payload, &[0x21, 0x00, 0x00, 0x01, 0xab]);
        assert_eq!(discard, 0);
    }

    #[test]
    fn handshake_flight() {
        // intended to be a realistic example
        let mut input = include_bytes!("../../testdata/handshake-test.1.bin").to_vec();
        let locator = Locator::new(&input);

        let mut hs = HandshakeDeframer::default();

        let mut iter = DeframerIter::new(&mut input[..]);

        while let Some(message) = iter.next() {
            let plain = message.unwrap().into_plain_message();
            std::println!("message {plain:?}");

            hs.input_message(plain, &locator, iter.bytes_consumed());
        }

        hs.coalesce(&mut input[..]).unwrap();

        let mut iter = hs.iter(&input[..]);
        for _ in 0..4 {
            let (msg, discard) = iter.next().unwrap();
            assert!(matches!(
                msg,
                InboundPlainMessage {
                    typ: ContentType::Handshake,
                    ..
                }
            ));
            assert_eq!(discard, 0);
        }

        let (msg, discard) = iter.next().unwrap();
        assert!(matches!(
            msg,
            InboundPlainMessage {
                typ: ContentType::Handshake,
                ..
            }
        ));
        assert_eq!(discard, 4280);
        drop(iter);

        input.drain(0..discard);
        assert!(input.is_empty());
    }
}
