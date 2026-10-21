#[test]
fn test_string_impls() {
    fn check_writeable_slice<W: Writeable + core::fmt::Display>(writeables: &[W]) {
        assert_writeable_eq!(&writeables[0], "");
        assert_writeable_eq!(&writeables[1], "abc");
        assert!(matches!(writeables[0].write_to_string(), Cow::Borrowed(_)));
        assert!(matches!(writeables[1].write_to_string(), Cow::Borrowed(_)));
    }

    // test str impl
    let arr: &[&str] = &["", "abc"];
    check_writeable_slice(arr);

    // test String impl
    let arr: &[String] = &[String::new(), "abc".to_owned()];
    check_writeable_slice(arr);

    // test char impl
    let chars = ['a', 'β', '你', '😀'];
    for i in 0..chars.len() {
        let s = String::from(chars[i]);
        assert_writeable_eq!(&chars[i], s);
        for j in 0..chars.len() {
            assert_eq!(
                cmp_str(&chars[j], &s),
                chars[j].cmp(&chars[i]),
                "{:?} vs {:?}",
                chars[j],
                chars[i]
            );
        }
    }

    // test Cow impl
    let arr: &[Cow<str>] = &[Cow::Borrowed(""), Cow::Owned("abc".to_string())];
    check_writeable_slice(arr);

    // test Box impl
    let arr: &[Box<str>] = &["".into(), "abc".into()];
    check_writeable_slice(arr);

    // test Rc impl
    let arr: &[alloc::rc::Rc<str>] = &["".into(), "abc".into()];
    check_writeable_slice(arr);

    // test Arc impl
    let arr: &[alloc::sync::Arc<str>] = &["".into(), "abc".into()];
    check_writeable_slice(arr);

    // test &T impl
    let arr: &[&String] = &[&String::new(), &"abc".to_owned()];
    check_writeable_slice(arr);
}
