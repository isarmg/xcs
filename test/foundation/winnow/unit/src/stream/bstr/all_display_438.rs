#[cfg(all(test, feature = "std"))]
mod display {
    use crate::stream::BStr;

    #[test]
    fn clean() {
        assert_eq!(&format!("{}", BStr::new(b"abc")), "abc");
        assert_eq!(&format!("{}", BStr::new(b"\xf0\x28\x8c\xbc")), "�(��");
    }
}
