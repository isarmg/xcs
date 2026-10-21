#[cfg(test)]
mod tests {
    #[no_implicit_prelude]
    mod hygiene {
        #[allow(unused_macros)]
        #[test]
        fn macro_hygiene() {
            // This `u64` is `!Sized`, so `cmsg_space!` will fail if it tries to get its size with
            // `size_of()`.
            #[allow(dead_code, non_camel_case_types)]
            struct u64([u8]);

            // Ensure that when `cmsg*_space!` calls itself recursively, it really calls itself and
            // not these macros.
            macro_rules! cmsg_space {
                ($($tt:tt)*) => {{
                    let v: usize = ::core::panic!("Wrong cmsg_space! macro called");
                    v
                }};
            }
            macro_rules! cmsg_aligned_space {
                ($($tt:tt)*) => {{
                    let v: usize = ::core::panic!("Wrong cmsg_aligned_space! macro called");
                    v
                }};
            }

            crate::cmsg_space!(ScmRights(1));
            crate::cmsg_space!(TxTime(1));
            #[cfg(linux_kernel)]
            {
                crate::cmsg_space!(ScmCredentials(1));
                crate::cmsg_space!(ScmRights(1), ScmCredentials(1), TxTime(1));
                crate::cmsg_aligned_space!(ScmRights(1), ScmCredentials(1), TxTime(1));
            }
        }
    }
}
