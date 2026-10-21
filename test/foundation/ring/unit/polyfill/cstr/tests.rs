mod tests {
    use super::const_from_bytes_with_nul;

    // Bad.
    const _EMPTY_UNTERMINATED: () = assert!(const_from_bytes_with_nul(b"").is_none());
    const _EMPTY_DOUBLE_TERMINATED: () = assert!(const_from_bytes_with_nul(b"\0\0").is_none());
    const _DOUBLE_NUL: () = assert!(const_from_bytes_with_nul(b"\0\0").is_none());
    const _LEADINGL_NUL: () = assert!(const_from_bytes_with_nul(b"\0a\0").is_none());
    const _INTERNAL_NUL_UNTERMINATED: () = assert!(const_from_bytes_with_nul(b"\0a").is_none());

    // Good.
    const _EMPTY_TERMINATED: () = assert!(const_from_bytes_with_nul(b"\0").is_some());
    const _NONEMPTY: () = assert!(const_from_bytes_with_nul(b"asdf\0").is_some());
    const _1_CHAR: () = assert!(const_from_bytes_with_nul(b"a\0").is_some());
}
