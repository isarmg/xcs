#[cfg(test)]
#[allow(unused)]
mod macro_checks {
    use core::prelude::v1::*;

    s! {
        pub struct S1 {
            pub a: u32,
            b: u32,
        }

        struct S1Priv {
            pub a: u32,
            b: u32,
        }
    }

    s_no_extra_traits! {
        pub struct S2 {
            pub a: u32,
            b: u32,
        }

        struct S2Priv {
            pub a: u32,
            b: u32,
        }

        pub union U2 {
            pub a: u32,
            b: f32,
        }

        union U2Priv {
            pub a: u32,
            b: f32,
        }
    }

    extern_ty! {
        type Foo;
        pub type Bar;
    }

    s2! {
        pub struct S3 {
            pub a: u32,
            #[custom_default([1; 64])]
            pub buf: [u8; 64],
        }

        struct S3Priv {
            pub a: u32,
            b: u32,
        }
    }

    s_no_extra_traits2! {
        pub union U3 {
            pub a: u32,
            b: f32,
        }

        pub struct S4 {
            pub a: u32,
            #[custom_default(unsafe { ::core::mem::zeroed::<U3>() })]
            pub u: U3,
        }
    }

    fn assert_impls_default<T: Default>() {}

    fn check_default() {
        assert_impls_default::<S3>();
        assert_impls_default::<S4>();
    }

    // Types configured out entirely, checking that the generated impls carry
    // the same `cfg` as the type. Without it they fail to compile with "cannot
    // find type".
    s2! {
        #[cfg(false)]
        pub struct S5 {
            pub a: u32,
            #[custom_default([1; 64])]
            pub buf: [u8; 64],
        }
    }

    s_no_extra_traits! {
        #[cfg(false)]
        pub union U4 {
            pub a: u32,
            b: f32,
        }
    }

    s_no_extra_traits2! {
        #[cfg(false)]
        pub union U5 {
            pub a: u32,
            b: f32,
        }

        #[cfg(false)]
        pub struct S6 {
            pub a: u32,
        }
    }

    // The generated impls name the type and its fields, so they need to allow
    // deprecation.  `deny` turns the warning into an error if that ever stops
    // being the case.
    mod deprecated_checks {
        #![deny(deprecated)]

        s2! {
            #[deprecated(since = "0.0.0", note = "check that generated impls don't warn")]
            pub struct S7 {
                pub a: u32,
            }
        }

        s_no_extra_traits! {
            #[deprecated(since = "0.0.0", note = "check that generated impls don't warn")]
            pub union U6 {
                pub a: u32,
                b: f32,
            }
        }

        s_no_extra_traits2! {
            #[deprecated(since = "0.0.0", note = "check that generated impls don't warn")]
            pub union U7 {
                pub a: u32,
                b: f32,
            }
        }
    }
}
