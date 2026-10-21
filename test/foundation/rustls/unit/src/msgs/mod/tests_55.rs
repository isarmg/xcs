#[cfg(test)]
mod tests {
    use super::codec::Reader;
    use super::message::{Message, OutboundOpaqueMessage};

    #[test]
    fn smoketest() {
        let bytes = include_bytes!("../testdata/handshake-test.1.bin");
        let mut r = Reader::init(bytes);

        while r.any_left() {
            let m = OutboundOpaqueMessage::read(&mut r).unwrap();

            let out = m.clone().encode();
            assert!(!out.is_empty());

            Message::try_from(m.into_plain_message()).unwrap();
        }
    }
}
