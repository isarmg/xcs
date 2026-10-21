#[cfg(test)]
mod tests {
    use std::format;

    use super::*;

    // these branches not reachable externally, unless something else goes wrong.
    #[test]
    fn test_read_in_new_state() {
        assert_eq!(
            format!("{:?}", EarlyDataState::default().read(&mut [0u8; 5])),
            "Err(Kind(BrokenPipe))"
        );
    }

    #[cfg(read_buf)]
    #[test]
    fn test_read_buf_in_new_state() {
        use core::io::BorrowedBuf;

        let mut buf = [0u8; 5];
        let mut buf: BorrowedBuf<'_, u8> = buf.as_mut_slice().into();
        assert_eq!(
            format!("{:?}", EarlyDataState::default().read_buf(buf.unfilled())),
            "Err(Kind(BrokenPipe))"
        );
    }
}
