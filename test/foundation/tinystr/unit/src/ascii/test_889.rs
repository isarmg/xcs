#[cfg(test)]
mod test {
    use super::*;
    use rand::distr::Distribution;
    use rand::distr::StandardUniform;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    const STRINGS: [&str; 26] = [
        "Latn",
        "laTn",
        "windows",
        "AR",
        "Hans",
        "macos",
        "AT",
        "infiniband",
        "FR",
        "en",
        "Cyrl",
        "FromIntegral",
        "NO",
        "419",
        "MacintoshOSX2019",
        "a3z",
        "A3z",
        "A3Z",
        "a3Z",
        "3A",
        "3Z",
        "3a",
        "3z",
        "@@[`{",
        "UK",
        "E12",
    ];

    fn gen_strings(num_strings: usize, allowed_lengths: &[usize]) -> Vec<String> {
        use rand::seq::IndexedRandom;
        let mut rng = SmallRng::seed_from_u64(2022);
        // Need to do this in 2 steps since the RNG is needed twice
        let string_lengths = core::iter::repeat_with(|| *allowed_lengths.choose(&mut rng).unwrap())
            .take(num_strings)
            .collect::<Vec<usize>>();
        string_lengths
            .iter()
            .map(|len| {
                StandardUniform
                    .sample_iter(&mut rng)
                    .filter(|b: &u8| *b > 0 && *b < 0x80)
                    .take(*len)
                    .collect::<Vec<u8>>()
            })
            .map(|byte_vec| String::from_utf8(byte_vec).expect("All ASCII"))
            .collect()
    }

    fn check_operation<T, F1, F2, const N: usize>(reference_f: F1, tinystr_f: F2)
    where
        F1: Fn(&str) -> T,
        F2: Fn(TinyAsciiStr<N>) -> T,
        T: fmt::Debug + PartialEq,
    {
        for s in STRINGS
            .into_iter()
            .map(str::to_owned)
            .chain(gen_strings(100, &[3, 4, 5, 8, 12]))
        {
            let t = match TinyAsciiStr::<N>::from_str(&s) {
                Ok(t) => t,
                Err(ParseError::TooLong { .. }) => continue,
                Err(e) => panic!("{}", e),
            };
            let expected = reference_f(&s);
            let actual = tinystr_f(t);
            assert_eq!(expected, actual, "TinyAsciiStr<{N}>: {s:?}");

            let s_utf16: Vec<u16> = s.encode_utf16().collect();
            let t = match TinyAsciiStr::<N>::try_from_utf16(&s_utf16) {
                Ok(t) => t,
                Err(ParseError::TooLong { .. }) => continue,
                Err(e) => panic!("{}", e),
            };
            let expected = reference_f(&s);
            let actual = tinystr_f(t);
            assert_eq!(expected, actual, "TinyAsciiStr<{N}>: {s:?}");
        }
    }

    #[test]
    fn test_is_ascii_alphabetic() {
        fn check<const N: usize>() {
            check_operation(
                |s| s.chars().all(|c| c.is_ascii_alphabetic()),
                |t: TinyAsciiStr<N>| TinyAsciiStr::is_ascii_alphabetic(&t),
            )
        }
        check::<2>();
        check::<3>();
        check::<4>();
        check::<5>();
        check::<8>();
        check::<16>();
    }

    #[test]
    fn test_is_ascii_alphanumeric() {
        fn check<const N: usize>() {
            check_operation(
                |s| s.chars().all(|c| c.is_ascii_alphanumeric()),
                |t: TinyAsciiStr<N>| TinyAsciiStr::is_ascii_alphanumeric(&t),
            )
        }
        check::<2>();
        check::<3>();
        check::<4>();
        check::<5>();
        check::<8>();
        check::<16>();
    }

    #[test]
    fn test_is_ascii_numeric() {
        fn check<const N: usize>() {
            check_operation(
                |s| s.chars().all(|c| c.is_ascii_digit()),
                |t: TinyAsciiStr<N>| TinyAsciiStr::is_ascii_numeric(&t),
            )
        }
        check::<2>();
        check::<3>();
        check::<4>();
        check::<5>();
        check::<8>();
        check::<16>();
    }

    #[test]
    fn test_is_ascii_lowercase() {
        fn check<const N: usize>() {
            check_operation(
                |s| {
                    s == TinyAsciiStr::<16>::try_from_str(s)
                        .unwrap()
                        .to_ascii_lowercase()
                        .as_str()
                },
                |t: TinyAsciiStr<N>| TinyAsciiStr::is_ascii_lowercase(&t),
            )
        }
        check::<2>();
        check::<3>();
        check::<4>();
        check::<5>();
        check::<8>();
        check::<16>();
    }

    #[test]
    fn test_is_ascii_titlecase() {
        fn check<const N: usize>() {
            check_operation(
                |s| {
                    s == TinyAsciiStr::<16>::try_from_str(s)
                        .unwrap()
                        .to_ascii_titlecase()
                        .as_str()
                },
                |t: TinyAsciiStr<N>| TinyAsciiStr::is_ascii_titlecase(&t),
            )
        }
        check::<2>();
        check::<3>();
        check::<4>();
        check::<5>();
        check::<8>();
        check::<16>();
    }

    #[test]
    fn test_is_ascii_uppercase() {
        fn check<const N: usize>() {
            check_operation(
                |s| {
                    s == TinyAsciiStr::<16>::try_from_str(s)
                        .unwrap()
                        .to_ascii_uppercase()
                        .as_str()
                },
                |t: TinyAsciiStr<N>| TinyAsciiStr::is_ascii_uppercase(&t),
            )
        }
        check::<2>();
        check::<3>();
        check::<4>();
        check::<5>();
        check::<8>();
        check::<16>();
    }

    #[test]
    fn test_is_ascii_alphabetic_lowercase() {
        fn check<const N: usize>() {
            check_operation(
                |s| {
                    // Check alphabetic
                    s.chars().all(|c| c.is_ascii_alphabetic()) &&
                    // Check lowercase
                    s == TinyAsciiStr::<16>::try_from_str(s)
                        .unwrap()
                        .to_ascii_lowercase()
                        .as_str()
                },
                |t: TinyAsciiStr<N>| TinyAsciiStr::is_ascii_alphabetic_lowercase(&t),
            )
        }
        check::<2>();
        check::<3>();
        check::<4>();
        check::<5>();
        check::<8>();
        check::<16>();
    }

    #[test]
    fn test_is_ascii_alphabetic_titlecase() {
        fn check<const N: usize>() {
            check_operation(
                |s| {
                    // Check alphabetic
                    s.chars().all(|c| c.is_ascii_alphabetic()) &&
                    // Check titlecase
                    s == TinyAsciiStr::<16>::try_from_str(s)
                        .unwrap()
                        .to_ascii_titlecase()
                        .as_str()
                },
                |t: TinyAsciiStr<N>| TinyAsciiStr::is_ascii_alphabetic_titlecase(&t),
            )
        }
        check::<2>();
        check::<3>();
        check::<4>();
        check::<5>();
        check::<8>();
        check::<16>();
    }

    #[test]
    fn test_is_ascii_alphabetic_uppercase() {
        fn check<const N: usize>() {
            check_operation(
                |s| {
                    // Check alphabetic
                    s.chars().all(|c| c.is_ascii_alphabetic()) &&
                    // Check uppercase
                    s == TinyAsciiStr::<16>::try_from_str(s)
                        .unwrap()
                        .to_ascii_uppercase()
                        .as_str()
                },
                |t: TinyAsciiStr<N>| TinyAsciiStr::is_ascii_alphabetic_uppercase(&t),
            )
        }
        check::<2>();
        check::<3>();
        check::<4>();
        check::<5>();
        check::<8>();
        check::<16>();
    }

    #[test]
    fn test_to_ascii_lowercase() {
        fn check<const N: usize>() {
            check_operation(
                |s| {
                    s.chars()
                        .map(|c| c.to_ascii_lowercase())
                        .collect::<String>()
                },
                |t: TinyAsciiStr<N>| TinyAsciiStr::to_ascii_lowercase(t).as_str().to_owned(),
            )
        }
        check::<2>();
        check::<3>();
        check::<4>();
        check::<5>();
        check::<8>();
        check::<16>();
    }

    #[test]
    fn test_to_ascii_titlecase() {
        fn check<const N: usize>() {
            check_operation(
                |s| {
                    let mut r = s
                        .chars()
                        .map(|c| c.to_ascii_lowercase())
                        .collect::<String>();
                    // Safe because the string is nonempty and an ASCII string
                    unsafe { r.as_bytes_mut()[0].make_ascii_uppercase() };
                    r
                },
                |t: TinyAsciiStr<N>| TinyAsciiStr::to_ascii_titlecase(t).as_str().to_owned(),
            )
        }
        check::<2>();
        check::<3>();
        check::<4>();
        check::<5>();
        check::<8>();
        check::<16>();
    }

    #[test]
    fn test_to_ascii_uppercase() {
        fn check<const N: usize>() {
            check_operation(
                |s| {
                    s.chars()
                        .map(|c| c.to_ascii_uppercase())
                        .collect::<String>()
                },
                |t: TinyAsciiStr<N>| TinyAsciiStr::to_ascii_uppercase(t).as_str().to_owned(),
            )
        }
        check::<2>();
        check::<3>();
        check::<4>();
        check::<5>();
        check::<8>();
        check::<16>();
    }

    #[test]
    fn lossy_constructor() {
        assert_eq!(TinyAsciiStr::<4>::from_utf8_lossy(b"", b'?').as_str(), "");
        assert_eq!(
            TinyAsciiStr::<4>::from_utf8_lossy(b"oh\0o", b'?').as_str(),
            "oh?o"
        );
        assert_eq!(
            TinyAsciiStr::<4>::from_utf8_lossy(b"\0", b'?').as_str(),
            "?"
        );
        assert_eq!(
            TinyAsciiStr::<4>::from_utf8_lossy(b"toolong", b'?').as_str(),
            "tool"
        );
        assert_eq!(
            TinyAsciiStr::<4>::from_utf8_lossy(&[b'a', 0x80, 0xFF, b'1'], b'?').as_str(),
            "a??1"
        );
    }

    #[test]
    #[should_panic(expected = "replacement must be a non-null ASCII byte")]
    fn from_utf8_lossy_rejects_non_ascii_replacement() {
        // A non-ASCII replacement byte (>= 0x80) would create an invalid AsciiByte
        // discriminant and then from_utf8_unchecked would be called on non-UTF-8
        // data (UB). The fix validates the replacement byte on entry.
        let _ = TinyAsciiStr::<4>::from_utf8_lossy(b"\xFF", 0xFF);
    }

    #[test]
    #[should_panic(expected = "replacement must be a non-null ASCII byte")]
    fn from_utf16_lossy_rejects_non_ascii_replacement() {
        let _ = TinyAsciiStr::<4>::from_utf16_lossy(&[0xFFFF], 0xFF);
    }

    #[test]
    #[should_panic(expected = "replacement must be a non-null ASCII byte")]
    fn from_utf8_lossy_rejects_null_replacement() {
        // Null (0x00) is also invalid — it's the TinyAsciiStr terminator.
        let _ = TinyAsciiStr::<4>::from_utf8_lossy(b"\xFF", 0x00);
    }
}
