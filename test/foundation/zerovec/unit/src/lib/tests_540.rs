#[cfg(test)]
// Expected sizes are based on a 64-bit architecture
#[cfg(target_pointer_width = "64")]
mod tests {
    use super::*;

    /// Checks that the size of the type is one of the given sizes.
    /// The size might differ across Rust versions or channels.
    macro_rules! check_size_of {
        ($sizes:pat, $type:path) => {
            assert!(
                matches!(size_of::<$type>(), $sizes),
                concat!(stringify!($type), " is of size {}"),
                size_of::<$type>()
            );
        };
    }

    #[test]
    fn check_sizes() {
        check_size_of!(24, ZeroVec<u8>);
        check_size_of!(24, ZeroVec<u32>);
        check_size_of!(32 | 24, VarZeroVec<[u8]>);
        check_size_of!(32 | 24, VarZeroVec<str>);
        check_size_of!(48, ZeroMap<u32, u32>);
        check_size_of!(56 | 48, ZeroMap<u32, str>);
        check_size_of!(56 | 48, ZeroMap<str, u32>);
        check_size_of!(64 | 48, ZeroMap<str, str>);
        check_size_of!(120 | 96, ZeroMap2d<str, str, str>);

        check_size_of!(24, Option<ZeroVec<u8>>);
        check_size_of!(32 | 24, Option<VarZeroVec<str>>);
        check_size_of!(64 | 56 | 48, Option<ZeroMap<str, str>>);
        check_size_of!(120 | 104 | 96, Option<ZeroMap2d<str, str, str>>);
    }

    #[test]
    fn test_miri_repro_eyepatch_hack_truncate() {
        let slice: &[u16] = &[1, 2, 3, 4];
        let zv: ZeroVec<u16> = ZeroVec::try_from_slice(slice).unwrap();
        let _truncated = zv.truncated(2);
    }

    #[test]
    fn test_miri_repro_encode_varule_to_box() {
        let str_slice: &str = "hello world";
        let _boxed: Box<str> = ule::encode_varule_to_box(str_slice);
    }

    #[test]
    fn test_miri_repro_varzerocow_new_owned() {
        let boxed_str: Box<str> = Box::from("hello world");
        let _cow: VarZeroCow<str> = VarZeroCow::new_owned(boxed_str);
    }
}
