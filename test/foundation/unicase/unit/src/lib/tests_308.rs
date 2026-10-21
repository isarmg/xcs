#[cfg(test)]
mod tests {
    use super::UniCase;
    use alloc::borrow::Cow;
    use std::borrow::ToOwned;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::string::String;

    fn hash<T: Hash>(t: &T) -> u64 {
        let mut s = DefaultHasher::new();
        t.hash(&mut s);
        s.finish()
    }

    #[test]
    fn test_copy_for_refs() {
        fn foo<T>(_: UniCase<T>) {}

        let a = UniCase::new("foobar");
        foo(a);
        foo(a);
    }

    #[test]
    fn test_eq_ascii() {
        let a = UniCase::new("foobar");
        let b = UniCase::new("FOOBAR");
        let c = UniCase::ascii("FoObAr");
        let d = UniCase::<&str>::from("foobar");

        assert_eq!(a, b);
        assert_eq!(a, c);
        assert_eq!(a, d);

        assert_eq!(b, a);
        assert_eq!(b, c);
        assert_eq!(b, d);

        assert_eq!(c, a);
        assert_eq!(c, b);
        assert_eq!(c, d);

        assert_eq!(d, a);
        assert_eq!(d, b);
        assert_eq!(d, c);

        assert_eq!(hash(&a), hash(&b));
        assert_eq!(hash(&a), hash(&c));
        assert_eq!(hash(&a), hash(&d));

        assert!(a.is_ascii());
        assert!(b.is_ascii());
        assert!(c.is_ascii());
        assert!(d.is_ascii());
    }

    #[test]
    fn test_str_ascii() {
        // https://github.com/seanmonstar/unicase/issues/76

        let a = UniCase::new("foobar");
        let b = UniCase::<&str>::from("foobar");
        let c = UniCase::<String>::from(String::from("foobar"));
        let d = UniCase::<Cow<str>>::from(Cow::from("foobar"));

        assert!(a.is_ascii());
        assert!(b.is_ascii());
        assert!(c.is_ascii());
        assert!(d.is_ascii());
    }

    #[test]
    fn test_eq_unicode() {
        let a = UniCase::new("στιγμας");
        let b = UniCase::new("στιγμασ");
        assert_eq!(a, b);
        assert_eq!(b, a);
        assert_eq!(hash(&a), hash(&b));
    }

    #[test]
    fn test_eq_unicode_left_is_substring() {
        // https://github.com/seanmonstar/unicase/issues/38
        let a = UniCase::unicode("foo");
        let b = UniCase::unicode("foobar");

        assert!(a != b);
        assert!(b != a);
    }

    #[cfg(feature = "nightly")]
    #[bench]
    fn bench_unicase_ascii(b: &mut ::test::Bencher) {
        b.bytes = b"foobar".len() as u64;
        let x = UniCase::new("foobar");
        let y = UniCase::new("FOOBAR");
        b.iter(|| assert_eq!(x, y));
    }

    #[cfg(feature = "nightly")]
    static SUBJECT: &'static [u8] = b"ffoo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz foo bar baz oo bar baz quux herp derp";

    #[cfg(feature = "nightly")]
    #[inline(never)]
    fn is_ascii(bytes: &[u8]) -> bool {
        #[allow(unused, deprecated)]
        use std::ascii::AsciiExt;
        bytes.is_ascii()
    }

    #[cfg(feature = "nightly")]
    #[bench]
    fn bench_is_ascii(b: &mut ::test::Bencher) {
        b.iter(|| assert!(is_ascii(SUBJECT)));
    }

    #[cfg(feature = "nightly")]
    #[bench]
    fn bench_is_utf8(b: &mut ::test::Bencher) {
        b.iter(|| assert!(::std::str::from_utf8(SUBJECT).is_ok()));
    }

    #[test]
    fn test_case_cmp() {
        assert!(UniCase::new("a") < UniCase::new("B"));

        assert!(UniCase::new("A") < UniCase::new("b"));
        assert!(UniCase::new("aa") > UniCase::new("a"));

        assert!(UniCase::new("a") < UniCase::new("aa"));
        assert!(UniCase::new("a") < UniCase::new("AA"));
    }

    #[test]
    fn test_from_impls() {
        let view: &'static str = "foobar";
        let _: UniCase<&'static str> = view.into();
        let _: UniCase<&str> = view.into();
        let _: UniCase<String> = view.into();

        let owned: String = view.to_owned();
        let _: UniCase<&str> = (&owned).into();
        let _: UniCase<String> = owned.into();
    }

    #[test]
    fn test_into_impls() {
        let view: UniCase<&'static str> = UniCase::new("foobar");
        let _: &'static str = view.into();
        let _: &str = view.into();

        let owned: UniCase<String> = "foobar".into();
        let _: String = owned.clone().into();
        let _: &str = owned.as_ref();
    }

    #[test]
    fn test_unicase_unicode_const() {
        const _UNICASE: UniCase<&'static str> = UniCase::unicode("");
    }
}
