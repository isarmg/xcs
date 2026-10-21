#[cfg(test)]
mod tests {
    use std::prelude::v1::*;
    use std::vec;

    use super::*;

    #[test]
    fn interrupted_length_prefixed_buffer_leaves_maximum_length() {
        let mut buf = Vec::new();
        let nested = LengthPrefixedBuffer::new(ListLength::U16, &mut buf);
        nested.buf.push(0xaa);
        assert_eq!(nested.buf, &vec![0xff, 0xff, 0xaa]);
        // <- if the buffer is accidentally read here, there is no possibility
        //    that the contents of the length-prefixed buffer are interpreted
        //    as a subsequent encoding (perhaps allowing injection of a different
        //    extension)
        drop(nested);
        assert_eq!(buf, vec![0x00, 0x01, 0xaa]);
    }
}
