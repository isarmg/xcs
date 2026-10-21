#[cfg(test)]
mod composite_rejection_tests {
    use self::defs::*;
    use crate::Error;
    use std::error::Error as _;

    #[allow(dead_code, unreachable_pub)]
    mod defs {
        __define_rejection! {
            #[status = BAD_REQUEST]
            #[body = "error message 1"]
            pub struct Inner1;
        }
        __define_rejection! {
            #[status = BAD_REQUEST]
            #[body = "error message 2"]
            pub struct Inner2(Error);
        }
        __composite_rejection! {
            pub enum Outer { Inner1, Inner2 }
        }
    }

    /// The implementation of `.source()` on `Outer` should defer straight to the implementation
    /// on its inner type instead of returning the inner type itself, because the `Display`
    /// implementation on `Outer` already forwards to the inner type and so it would result in two
    /// errors in the chain `Display`ing the same thing.
    #[test]
    fn source_gives_inner_source() {
        let rejection = Outer::Inner1(Inner1);
        assert!(rejection.source().is_none());

        let msg = "hello world";
        let rejection = Outer::Inner2(Inner2(Error::new(msg)));
        assert_eq!(rejection.source().unwrap().to_string(), msg);
    }
}
