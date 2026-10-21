#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_encoding_size() {
        assert_eq!(encoding_size(624485), 3);
    }

    #[test]
    fn it_encodes_an_empty_module() {
        let bytes = Module::new().finish();
        assert_eq!(bytes, [0x00, b'a', b's', b'm', 0x01, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn it_encodes_an_empty_component() {
        let bytes = Component::new().finish();
        assert_eq!(bytes, [0x00, b'a', b's', b'm', 0x0d, 0x00, 0x01, 0x00]);
    }
}
