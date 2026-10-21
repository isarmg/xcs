#[cfg(any(feature = "std", not(no_core_net)))]
#[test]
fn test_format_u8() {
    let mut i = 0u8;

    loop {
        let mut buf = [0u8; 3];
        let written = format_u8(i, &mut buf);
        assert_eq!(i.to_string().as_bytes(), &buf[..written]);

        match i.checked_add(1) {
            Some(next) => i = next,
            None => break,
        }
    }
}
