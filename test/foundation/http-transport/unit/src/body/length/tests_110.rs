#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sub_if_known() {
        let mut len = DecodedLength::new(30);
        len.sub_if(20);

        assert_eq!(len.0, 10);
    }

    #[test]
    fn sub_if_chunked() {
        let mut len = DecodedLength::CHUNKED;
        len.sub_if(20);

        assert_eq!(len, DecodedLength::CHUNKED);
    }
}
