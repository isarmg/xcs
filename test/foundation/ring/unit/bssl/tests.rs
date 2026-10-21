#[cfg(test)]
mod tests {
    mod result {
        use crate::bssl;
        use core::{
            ffi::c_int,
            mem::{align_of, size_of},
        };

        #[test]
        fn size_and_alignment() {
            type Underlying = c_int;
            assert_eq!(size_of::<bssl::Result>(), size_of::<Underlying>());
            assert_eq!(align_of::<bssl::Result>(), align_of::<Underlying>());
        }

        #[test]
        fn semantics() {
            assert!(Result::from(bssl::Result(0)).is_err());
            assert!(Result::from(bssl::Result(1)).is_ok());
        }
    }
}
