#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::c;

    #[test]
    fn test_layouts() {
        static_assertions::assert_eq_size!(SocketAddrLen, c::socklen_t);

        #[cfg(not(any(windows, target_os = "redox")))]
        assert_eq!(
            memoffset::span_of!(c::msghdr, msg_namelen).len(),
            size_of::<SocketAddrLen>()
        );

        assert!(size_of::<SocketAddrLen>() <= size_of::<usize>());
    }
}
