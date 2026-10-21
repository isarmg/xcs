#[test]
fn in_addr_convertion() {
    let ip = Ipv4Addr::new(127, 0, 0, 1);
    let raw = to_in_addr(&ip);
    // NOTE: `in_addr` is packed on NetBSD and it's unsafe to borrow.
    let a = raw.s_addr;
    assert_eq!(a, u32::from_ne_bytes([127, 0, 0, 1]));
    assert_eq!(from_in_addr(raw), ip);

    let ip = Ipv4Addr::new(127, 34, 4, 12);
    let raw = to_in_addr(&ip);
    let a = raw.s_addr;
    assert_eq!(a, u32::from_ne_bytes([127, 34, 4, 12]));
    assert_eq!(from_in_addr(raw), ip);
}
