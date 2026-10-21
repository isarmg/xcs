#[cfg(all(test, not(crossbeam_loom)))]
mod tests {
    use super::{Atomic, Owned, Shared};
    use std::{format, mem::MaybeUninit};

    #[test]
    fn valid_tag_i8() {
        Shared::<i8>::null().with_tag(0);
    }

    #[test]
    fn valid_tag_i64() {
        Shared::<i64>::null().with_tag(7);
    }

    #[test]
    fn const_null() {
        use super::{Atomic, Shared};
        static _A: Atomic<u8> = Atomic::<u8>::null();
        static _S: () = {
            let _shared = Shared::<u8>::null();
        };
    }

    #[test]
    fn array_init() {
        let mut owned = Owned::<[MaybeUninit<usize>]>::init(10);
        let arr: &mut [MaybeUninit<usize>] = &mut owned;
        arr[arr.len() - 1].write(20);
        assert_eq!(arr.len(), 10);
    }

    #[test]
    fn format_null() {
        let atomic = Atomic::<usize>::null();
        assert_eq!(format!("{atomic:p}"), "0x0");
        let atomic = Atomic::<[MaybeUninit<usize>]>::null();
        assert_eq!(format!("{atomic:p}"), "0x0");

        let shared = Shared::<usize>::null();
        assert_eq!(format!("{shared:p}"), "0x0");
        let shared = Shared::<[MaybeUninit<usize>]>::null();
        assert_eq!(format!("{shared:p}"), "0x0");
    }
}
