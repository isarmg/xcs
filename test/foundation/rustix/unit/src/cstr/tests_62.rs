#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_cstr() {
        use crate::ffi::CString;
        use alloc::borrow::ToOwned as _;
        assert_eq!(cstr!(""), &*CString::new("").unwrap());
        assert_eq!(cstr!("").to_owned(), CString::new("").unwrap());
        assert_eq!(cstr!("hello"), &*CString::new("hello").unwrap());
        assert_eq!(cstr!("hello").to_owned(), CString::new("hello").unwrap());
    }

    #[test]
    #[should_panic]
    fn test_invalid_cstr() {
        let _ = cstr!("hello\0world");
    }

    #[test]
    #[should_panic]
    fn test_invalid_empty_cstr() {
        let _ = cstr!("\0");
    }

    #[no_implicit_prelude]
    mod hygiene {
        #[allow(unused_macros)]
        #[test]
        fn macro_hygiene() {
            macro_rules! assert {
                ($($tt:tt)*) => {
                    ::core::panic!("cstr! called the wrong assert! macro");
                };
            }
            macro_rules! concat {
                ($($tt:tt)*) => {{
                    let v: &str = ::core::panic!("cstr! called the wrong concat! macro");
                    v
                }};
            }

            let _ = cstr!("foo");
        }
    }
}
