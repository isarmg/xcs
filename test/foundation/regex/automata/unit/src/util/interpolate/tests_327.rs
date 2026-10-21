#[cfg(test)]
mod tests {
    use alloc::{string::String, vec, vec::Vec};

    use super::{find_cap_ref, CaptureRef};

    macro_rules! find {
        ($name:ident, $text:expr) => {
            #[test]
            fn $name() {
                assert_eq!(None, find_cap_ref($text.as_bytes()));
            }
        };
        ($name:ident, $text:expr, $capref:expr) => {
            #[test]
            fn $name() {
                assert_eq!(Some($capref), find_cap_ref($text.as_bytes()));
            }
        };
    }

    macro_rules! c {
        ($name_or_number:expr, $pos:expr) => {
            CaptureRef { cap: $name_or_number.into(), end: $pos }
        };
    }

    find!(find_cap_ref1, "$foo", c!("foo", 4));
    find!(find_cap_ref2, "${foo}", c!("foo", 6));
    find!(find_cap_ref3, "$0", c!(0, 2));
    find!(find_cap_ref4, "$5", c!(5, 2));
    find!(find_cap_ref5, "$10", c!(10, 3));
    // See https://github.com/rust-lang/regex/pull/585
    // for more on characters following numbers
    find!(find_cap_ref6, "$42a", c!("42a", 4));
    find!(find_cap_ref7, "${42}a", c!(42, 5));
    find!(find_cap_ref8, "${42");
    find!(find_cap_ref9, "${42 ");
    find!(find_cap_ref10, " $0 ");
    find!(find_cap_ref11, "$");
    find!(find_cap_ref12, " ");
    find!(find_cap_ref13, "");
    find!(find_cap_ref14, "$1-$2", c!(1, 2));
    find!(find_cap_ref15, "$1_$2", c!("1_", 3));
    find!(find_cap_ref16, "$x-$y", c!("x", 2));
    find!(find_cap_ref17, "$x_$y", c!("x_", 3));
    find!(find_cap_ref18, "${#}", c!("#", 4));
    find!(find_cap_ref19, "${Z[}", c!("Z[", 5));
    find!(find_cap_ref20, "${¾}", c!("¾", 5));
    find!(find_cap_ref21, "${¾a}", c!("¾a", 6));
    find!(find_cap_ref22, "${a¾}", c!("a¾", 6));
    find!(find_cap_ref23, "${☃}", c!("☃", 6));
    find!(find_cap_ref24, "${a☃}", c!("a☃", 7));
    find!(find_cap_ref25, "${☃a}", c!("☃a", 7));
    find!(find_cap_ref26, "${名字}", c!("名字", 9));

    fn interpolate_string(
        mut name_to_index: Vec<(&'static str, usize)>,
        caps: Vec<&'static str>,
        replacement: &str,
    ) -> String {
        name_to_index.sort_by_key(|x| x.0);

        let mut dst = String::new();
        super::string(
            replacement,
            |i, dst| {
                if let Some(&s) = caps.get(i) {
                    dst.push_str(s);
                }
            },
            |name| -> Option<usize> {
                name_to_index
                    .binary_search_by_key(&name, |x| x.0)
                    .ok()
                    .map(|i| name_to_index[i].1)
            },
            &mut dst,
        );
        dst
    }

    fn interpolate_bytes(
        mut name_to_index: Vec<(&'static str, usize)>,
        caps: Vec<&'static str>,
        replacement: &str,
    ) -> String {
        name_to_index.sort_by_key(|x| x.0);

        let mut dst = vec![];
        super::bytes(
            replacement.as_bytes(),
            |i, dst| {
                if let Some(&s) = caps.get(i) {
                    dst.extend_from_slice(s.as_bytes());
                }
            },
            |name| -> Option<usize> {
                name_to_index
                    .binary_search_by_key(&name, |x| x.0)
                    .ok()
                    .map(|i| name_to_index[i].1)
            },
            &mut dst,
        );
        String::from_utf8(dst).unwrap()
    }

    macro_rules! interp {
        ($name:ident, $map:expr, $caps:expr, $hay:expr, $expected:expr $(,)*) => {
            #[test]
            fn $name() {
                assert_eq!(
                    $expected,
                    interpolate_string($map, $caps, $hay),
                    "interpolate::string failed",
                );
                assert_eq!(
                    $expected,
                    interpolate_bytes($map, $caps, $hay),
                    "interpolate::bytes failed",
                );
            }
        };
    }

    interp!(
        interp1,
        vec![("foo", 2)],
        vec!["", "", "xxx"],
        "test $foo test",
        "test xxx test",
    );

    interp!(
        interp2,
        vec![("foo", 2)],
        vec!["", "", "xxx"],
        "test$footest",
        "test",
    );

    interp!(
        interp3,
        vec![("foo", 2)],
        vec!["", "", "xxx"],
        "test${foo}test",
        "testxxxtest",
    );

    interp!(
        interp4,
        vec![("foo", 2)],
        vec!["", "", "xxx"],
        "test$2test",
        "test",
    );

    interp!(
        interp5,
        vec![("foo", 2)],
        vec!["", "", "xxx"],
        "test${2}test",
        "testxxxtest",
    );

    interp!(
        interp6,
        vec![("foo", 2)],
        vec!["", "", "xxx"],
        "test $$foo test",
        "test $foo test",
    );

    interp!(
        interp7,
        vec![("foo", 2)],
        vec!["", "", "xxx"],
        "test $foo",
        "test xxx",
    );

    interp!(
        interp8,
        vec![("foo", 2)],
        vec!["", "", "xxx"],
        "$foo test",
        "xxx test",
    );

    interp!(
        interp9,
        vec![("bar", 1), ("foo", 2)],
        vec!["", "yyy", "xxx"],
        "test $bar$foo",
        "test yyyxxx",
    );

    interp!(
        interp10,
        vec![("bar", 1), ("foo", 2)],
        vec!["", "yyy", "xxx"],
        "test $ test",
        "test $ test",
    );

    interp!(
        interp11,
        vec![("bar", 1), ("foo", 2)],
        vec!["", "yyy", "xxx"],
        "test ${} test",
        "test  test",
    );

    interp!(
        interp12,
        vec![("bar", 1), ("foo", 2)],
        vec!["", "yyy", "xxx"],
        "test ${ } test",
        "test  test",
    );

    interp!(
        interp13,
        vec![("bar", 1), ("foo", 2)],
        vec!["", "yyy", "xxx"],
        "test ${a b} test",
        "test  test",
    );

    interp!(
        interp14,
        vec![("bar", 1), ("foo", 2)],
        vec!["", "yyy", "xxx"],
        "test ${a} test",
        "test  test",
    );

    // This is a funny case where a braced reference is never closed, but
    // within the unclosed braced reference, there is an unbraced reference.
    // In this case, the braced reference is just treated literally and the
    // unbraced reference is found.
    interp!(
        interp15,
        vec![("bar", 1), ("foo", 2)],
        vec!["", "yyy", "xxx"],
        "test ${wat $bar ok",
        "test ${wat yyy ok",
    );
}
