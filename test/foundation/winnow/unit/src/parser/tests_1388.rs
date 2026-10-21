#[cfg(test)]
mod tests {
    use super::*;

    use snapbox::prelude::*;
    use snapbox::str;

    use crate::binary::be_u16;
    use crate::error::ErrMode;
    use crate::error::Needed;
    use crate::error::TestResult;
    use crate::token::take;
    use crate::Partial;

    #[doc(hidden)]
    #[macro_export]
    macro_rules! assert_size (
        ($t:ty, $sz:expr) => (
            assert!(core::mem::size_of::<$t>() <= $sz, "{} <= {} failed", core::mem::size_of::<$t>(), $sz);
        );
    );

    #[test]
    #[cfg(target_pointer_width = "64")]
    fn size_test() {
        assert_size!(Result<&[u8], (&[u8], u32)>, 40);
        assert_size!(Result<&str, u32>, 40);
        assert_size!(Needed, 8);
        assert_size!(ErrMode<u32>, 16);
    }

    #[test]
    fn err_map_test() {
        let e = ErrMode::Backtrack(1);
        assert_eq!(e.map(|v| v + 1), ErrMode::Backtrack(2));
    }

    #[test]
    fn single_element_tuples() {
        use crate::ascii::alpha1;

        let mut parser = (alpha1,);
        assert_parse!(
            parser.parse_peek("abc123def"),
            str![[r#"
Ok(
    (
        "123def",
        (
            "abc",
        ),
    ),
)

"#]]
            .raw()
        );
        assert_parse!(
            parser.parse_peek("123def"),
            str![[r#"
Err(
    Backtrack(
        InputError {
            input: "123def",
        },
    ),
)

"#]]
            .raw()
        );
    }

    #[test]
    fn tuple_test() {
        #[allow(clippy::type_complexity)]
        fn tuple_3<'i>(
            i: &mut Partial<&'i [u8]>,
        ) -> TestResult<Partial<&'i [u8]>, (u16, &'i [u8], &'i [u8])> {
            (be_u16, take(3u8), "fg").parse_next(i)
        }

        assert_parse!(
            tuple_3.parse_peek(Partial::new(&b"abcdefgh"[..])),
            str![[r#"
Ok(
    (
        Partial {
            input: [
                104,
            ],
            partial: true,
        },
        (
            24930,
            [
                99,
                100,
                101,
            ],
            [
                102,
                103,
            ],
        ),
    ),
)

"#]]
            .raw()
        );
        assert_parse!(
            tuple_3.parse_peek(Partial::new(&b"abcd"[..])),
            str![[r#"
Err(
    Incomplete(
        Size(
            1,
        ),
    ),
)

"#]]
            .raw()
        );
        assert_parse!(
            tuple_3.parse_peek(Partial::new(&b"abcde"[..])),
            str![[r#"
Err(
    Incomplete(
        Unknown,
    ),
)

"#]]
            .raw()
        );
        assert_parse!(
            tuple_3.parse_peek(Partial::new(&b"abcdejk"[..])),
            str![[r#"
Err(
    Backtrack(
        InputError {
            input: Partial {
                input: [
                    106,
                    107,
                ],
                partial: true,
            },
        },
    ),
)

"#]]
            .raw()
        );
    }

    #[test]
    fn unit_type() {
        fn parser<'i>(i: &mut &'i str) -> TestResult<&'i str, ()> {
            ().parse_next(i)
        }
        assert_parse!(
            parser.parse_peek("abxsbsh"),
            str![[r#"
Ok(
    (
        "abxsbsh",
        (),
    ),
)

"#]]
            .raw()
        );
        assert_parse!(
            parser.parse_peek("sdfjakdsas"),
            str![[r#"
Ok(
    (
        "sdfjakdsas",
        (),
    ),
)

"#]]
            .raw()
        );
        assert_parse!(
            parser.parse_peek(""),
            str![[r#"
Ok(
    (
        "",
        (),
    ),
)

"#]]
            .raw()
        );
    }
}
