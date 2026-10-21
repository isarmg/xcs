#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sizes() {
        static_assertions::assert_eq_size!(RawUid, u32);
        static_assertions::assert_eq_size!(RawGid, u32);
        static_assertions::assert_eq_size!(RawUid, libc::uid_t);
        static_assertions::assert_eq_size!(RawGid, libc::gid_t);
    }
}
